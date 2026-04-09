//! Display-width helpers for terminal layout (ANSI-aware + Unicode wide chars).
//!
//! All functions operate on *visible* columns — ANSI CSI escape sequences are
//! stripped before measuring, and Unicode double-width characters count as 2.
//!
//! # Platform notes
//! [`terminal_cols`] reads from an [`AtomicUsize`] cache that is seeded by
//! [`start_resize_monitor`] and updated in-band by the editor's resize handler
//! via [`update_terminal_cols`].  On Windows the crossterm back-end handles
//! raw-mode and `terminal::size` correctly; no extra adaptation is needed here.

use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

static TERMINAL_COLS: AtomicUsize = AtomicUsize::new(80);

/// Return the cached terminal column count.
///
/// The value is seeded by [`start_resize_monitor`] and kept up-to-date by
/// both the polling thread and the editor's inline `Event::Resize` handler.
pub(crate) fn terminal_cols() -> usize {
    TERMINAL_COLS.load(Ordering::Relaxed)
}

/// Immediately update the cached column count.
///
/// Called from the editor's `Event::Resize` handler so the change takes effect
/// before the next frame is drawn, without waiting for the polling thread.
pub(crate) fn update_terminal_cols(cols: usize) {
    TERMINAL_COLS.store(cols, Ordering::Relaxed);
}

/// Seed the terminal-width cache from the real terminal and spawn a background
/// thread that polls [`crossterm::terminal::size`] every 150 ms.
///
/// The polling thread acts as a fallback for environments where resize events
/// are not delivered (e.g. some Windows terminal emulators, CI pipes).
pub(crate) fn start_resize_monitor() {
    fn query() -> usize {
        crossterm::terminal::size().map_or(80, |(w, _)| w as usize)
    }
    TERMINAL_COLS.store(query(), Ordering::Relaxed);
    let _ = std::thread::Builder::new()
        .name("terminal-resize-monitor".into())
        .spawn(|| {
            let mut last = TERMINAL_COLS.load(Ordering::Relaxed);
            loop {
                std::thread::sleep(Duration::from_millis(150));
                let cols = query();
                if cols != last {
                    TERMINAL_COLS.store(cols, Ordering::Relaxed);
                    last = cols;
                }
            }
        });
}

/// Strip CSI `ESC [ … <final-byte>` sequences so the result contains only
/// printable characters.  Other escape sequences (OSC, DCS, …) are left
/// intact because they are uncommon in prompt text.
pub(crate) fn strip_ansi(s: &str) -> Cow<'_, str> {
    if !s.as_bytes().contains(&0x1b) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && matches!(chars.peek(), Some('[')) {
            chars.next(); // consume '['
                          // Consume parameter/intermediate bytes up to and including the
                          // final byte (0x40–0x7E).
            for x in chars.by_ref() {
                if (0x40..=0x7e).contains(&(x as u32)) {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
}

/// Visible display width of `s` (ANSI-stripped, Unicode-aware).
#[must_use]
pub(crate) fn display_width(s: &str) -> usize {
    strip_ansi(s).width()
}

/// Truncate `s` so its visible width is at most `max` columns.
///
/// If truncation occurs the last visible character is replaced with `…`
/// (U+2026 HORIZONTAL ELLIPSIS, 1 column wide).
#[must_use]
pub(crate) fn truncate_display(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if display_width(s) <= max {
        return s.to_string();
    }
    let mut out = String::new();
    let mut w = 0usize;
    for ch in strip_ansi(s).chars() {
        let cw = ch.width().unwrap_or(0);
        if w + cw > max.saturating_sub(1) {
            out.push('…');
            break;
        }
        w += cw;
        out.push(ch);
    }
    out
}

/// Pad or truncate `s` to exactly `target` visible columns.
///
/// Used to fill framed inner lines so each row has a uniform width.
#[must_use]
pub(crate) fn fit_display_width(s: &str, target: usize) -> String {
    let w = display_width(s);
    match w.cmp(&target) {
        std::cmp::Ordering::Less => format!("{s}{}", " ".repeat(target - w)),
        std::cmp::Ordering::Greater => truncate_display(s, target),
        std::cmp::Ordering::Equal => s.to_string(),
    }
}

/// Break `s` into lines each at most `max_width` visible columns wide.
///
/// Wrapping prefers soft break-points (`' '` and `'/'`) so that paths split
/// at natural boundaries.  Hard cuts are used only when no soft break-point
/// exists within the chunk.  Leading spaces on continuation lines are trimmed.
///
/// Returns at least one element; an empty `s` returns `vec![""]`.
#[must_use]
pub(crate) fn wrap_by_display_width(s: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![String::new()];
    }
    let plain = strip_ansi(s);
    if plain.width() <= max_width {
        return vec![s.to_string()];
    }

    let mut lines: Vec<String> = Vec::new();
    let mut remaining: &str = plain.as_ref();

    while !remaining.is_empty() {
        if remaining.width() <= max_width {
            lines.push(remaining.to_string());
            break;
        }

        let mut w = 0usize;
        let mut hard_cut = remaining.len();
        let mut last_soft: Option<usize> = None;

        for (byte_off, ch) in remaining.char_indices() {
            let cw = ch.width().unwrap_or(0);
            if w + cw > max_width {
                hard_cut = byte_off;
                break;
            }
            w += cw;
            if matches!(ch, ' ' | '/') {
                last_soft = Some(byte_off + ch.len_utf8());
            }
        }

        // Prefer the soft break when it sits in the last quarter of the chunk
        // so the first line isn't embarrassingly short.
        let cut = match last_soft {
            Some(pos) if pos >= hard_cut.saturating_sub(hard_cut / 4) => pos,
            _ => hard_cut,
        };

        lines.push(remaining[..cut].to_string());
        remaining = remaining[cut..].trim_start_matches(' ');
    }

    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── strip_ansi ───────────────────────────────────────────────────────────

    #[test]
    fn strip_ansi_plain_string_unchanged() {
        assert_eq!(strip_ansi("hello world"), "hello world");
    }

    #[test]
    fn strip_ansi_removes_color_codes() {
        let s = "\x1b[1;31mhello\x1b[0m";
        assert_eq!(strip_ansi(s), "hello");
    }

    #[test]
    fn strip_ansi_multiple_sequences() {
        let s = "\x1b[1mA\x1b[32mB\x1b[0mC";
        assert_eq!(strip_ansi(s), "ABC");
    }

    #[test]
    fn strip_ansi_adjacent_sequences() {
        let s = "\x1b[1m\x1b[4munderline bold\x1b[0m";
        assert_eq!(strip_ansi(s), "underline bold");
    }

    #[test]
    fn strip_ansi_no_escape_borrows_original() {
        let s = "no escape here";
        assert!(matches!(strip_ansi(s), Cow::Borrowed(_)));
    }

    #[test]
    fn strip_ansi_with_escape_allocates() {
        let s = "\x1b[31mred\x1b[0m";
        assert!(matches!(strip_ansi(s), Cow::Owned(_)));
    }

    #[test]
    fn strip_ansi_empty_string() {
        assert_eq!(strip_ansi(""), "");
    }

    // ── display_width ────────────────────────────────────────────────────────

    #[test]
    fn display_width_ascii() {
        assert_eq!(display_width("hello"), 5);
    }

    #[test]
    fn display_width_ignores_ansi() {
        assert_eq!(display_width("\x1b[31mhello\x1b[0m"), 5);
    }

    #[test]
    fn display_width_unicode_wide_chars() {
        assert_eq!(display_width("你好"), 4);
    }

    #[test]
    fn display_width_mixed() {
        assert_eq!(display_width("\x1b[1m你好\x1b[0m world"), 4 + 6);
    }

    #[test]
    fn display_width_empty() {
        assert_eq!(display_width(""), 0);
    }

    // ── truncate_display ─────────────────────────────────────────────────────

    #[test]
    fn truncate_display_fits_within_max() {
        assert_eq!(truncate_display("hello", 10), "hello");
    }

    #[test]
    fn truncate_display_exact_max_unchanged() {
        assert_eq!(truncate_display("hello", 5), "hello");
    }

    #[test]
    fn truncate_display_adds_ellipsis() {
        let result = truncate_display("hello world", 7);
        assert_eq!(display_width(&result), 7);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn truncate_display_zero_returns_empty() {
        assert_eq!(truncate_display("hello", 0), "");
    }

    #[test]
    fn truncate_display_one_returns_ellipsis() {
        let result = truncate_display("abc", 1);
        assert_eq!(result, "…");
    }

    #[test]
    fn truncate_display_wide_chars_counted_correctly() {
        let result = truncate_display("你好", 3);
        assert_eq!(display_width(&result), 3);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn truncate_display_strips_ansi_before_measuring() {
        let colored = "\x1b[31mhello world\x1b[0m";
        let result = truncate_display(colored, 7);
        assert_eq!(display_width(&result), 7);
    }

    // ── fit_display_width ────────────────────────────────────────────────────

    #[test]
    fn fit_display_width_pads_short() {
        let result = fit_display_width("hi", 5);
        assert_eq!(result, "hi   ");
        assert_eq!(display_width(&result), 5);
    }

    #[test]
    fn fit_display_width_truncates_long() {
        let result = fit_display_width("hello world", 5);
        assert_eq!(display_width(&result), 5);
    }

    #[test]
    fn fit_display_width_exact_unchanged() {
        let result = fit_display_width("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn fit_display_width_zero_returns_empty() {
        assert_eq!(fit_display_width("hello", 0), "");
    }

    #[test]
    fn fit_display_width_ansi_string_measures_visible_only() {
        let result = fit_display_width("\x1b[31mhi\x1b[0m", 5);
        assert_eq!(display_width(&result), 5);
    }

    // ── wrap_by_display_width ────────────────────────────────────────────────

    #[test]
    fn wrap_fits_in_one_line() {
        let lines = wrap_by_display_width("hello", 10);
        assert_eq!(lines, vec!["hello"]);
    }

    #[test]
    fn wrap_empty_string() {
        let lines = wrap_by_display_width("", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "");
    }

    #[test]
    fn wrap_zero_width_returns_empty_line() {
        let lines = wrap_by_display_width("abc", 0);
        assert_eq!(lines, vec![""]);
    }

    #[test]
    fn wrap_long_path_splits_at_slash() {
        // "aineer --resume ~/.aineer/sessions/session-12345"
        // With width=30 the soft break at '/' should be preferred.
        let s = "aineer --resume ~/.aineer/sessions/session-12345";
        let lines = wrap_by_display_width(s, 30);
        assert!(lines.len() > 1, "should wrap to multiple lines");
        for line in &lines {
            assert!(
                display_width(line) <= 30,
                "line too wide: {line:?} ({} cols)",
                display_width(line)
            );
        }
        // Full content is preserved (concatenated, ignoring trimmed leading spaces).
        let rejoined: String = lines.join("");
        assert!(
            rejoined.contains("session-12345"),
            "session ID must survive wrapping"
        );
    }

    #[test]
    fn wrap_hard_cut_when_no_soft_break() {
        // A string with no spaces or slashes must still wrap without panicking.
        let s = "abcdefghijklmnopqrstuvwxyz";
        let lines = wrap_by_display_width(s, 10);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(display_width(line) <= 10);
        }
    }

    #[test]
    fn wrap_all_lines_within_max_width() {
        let s = "aineer --resume ~/.aineer/sessions/session-1775379683907";
        for width in [20, 30, 38, 50, 80] {
            let lines = wrap_by_display_width(s, width);
            for line in &lines {
                assert!(
                    display_width(line) <= width,
                    "width={width}: line {line:?} exceeds limit"
                );
            }
        }
    }
}
