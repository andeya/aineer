//! Display-width helpers for terminal layout (ANSI + Unicode wide chars).

use std::borrow::Cow;

use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

/// Strip CSI `ESC [ ... final` sequences so width matches visible cells.
pub(crate) fn strip_ansi(s: &str) -> Cow<'_, str> {
    if !s.as_bytes().contains(&0x1b) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            if matches!(it.peek(), Some('[')) {
                it.next();
                for x in it.by_ref() {
                    if (0x40..=0x7e).contains(&(x as u32)) {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
}

#[must_use]
pub(crate) fn display_width(s: &str) -> usize {
    strip_ansi(s).width()
}

/// Truncate so visible width is at most `max` (ellipsis if shortened).
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

/// Pad or truncate to exactly `target` display width (used for framed inner lines).
#[must_use]
pub(crate) fn fit_display_width(s: &str, target: usize) -> String {
    let w = display_width(s);
    if w < target {
        format!("{s}{}", " ".repeat(target - w))
    } else if w > target {
        truncate_display(s, target)
    } else {
        s.to_string()
    }
}
