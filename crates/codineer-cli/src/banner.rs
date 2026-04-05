//! Welcome panel (Claude Code-style: border-title + two-column layout).

use std::path::Path;

use crate::style::Palette;
use crate::terminal_width::{display_width, fit_display_width, truncate_display};

/// Inner width of framed lines (between side borders), shared with REPL chrome below the banner.
pub(crate) const BANNER_INNER_WIDTH: usize = 76;

const LEFT_COL: usize = 36;
const DIVIDER: usize = 3; // " │ "
const RIGHT_COL: usize = BANNER_INNER_WIDTH - LEFT_COL - DIVIDER;
const LOGO_WIDTH: usize = 12;

pub(crate) struct BannerContext<'a> {
    pub workspace_summary: &'a str,
    pub cwd_display: &'a str,
    pub model: &'a str,
    pub permissions: &'a str,
    pub session_id: &'a str,
    pub session_path: &'a Path,
    pub has_codineer_md: bool,
}

pub(crate) fn welcome_banner(color: bool, ctx: BannerContext<'_>) -> String {
    let p = Palette::new(color);
    let version = crate::VERSION;

    let top = border_top(color, &p, version);
    let bot = border_bottom(color, &p);
    let left = left_column(color, &p, &ctx);
    let right = right_column(color, &p, &ctx);

    let div = if color {
        format!(" {}│{} ", p.gray, p.r)
    } else {
        " | ".to_string()
    };

    let row_count = left.len().max(right.len());
    let mut rows = Vec::with_capacity(row_count + 2);
    rows.push(top);
    for i in 0..row_count {
        let l = left.get(i).map(String::as_str).unwrap_or("");
        let r = right.get(i).map(String::as_str).unwrap_or("");
        let lp = fit_display_width(l, LEFT_COL);
        let rp = fit_display_width(r, RIGHT_COL);
        let inner = fit_display_width(&format!("{lp}{div}{rp}"), BANNER_INNER_WIDTH);
        rows.push(border_row(color, &p, &inner));
    }
    rows.push(bot);
    rows.join("\n")
}

fn center_in(text: &str, width: usize) -> String {
    let w = display_width(text);
    if w >= width {
        return truncate_display(text, width);
    }
    let pad_left = (width - w) / 2;
    let pad_right = width - w - pad_left;
    format!("{}{}{}", " ".repeat(pad_left), text, " ".repeat(pad_right))
}

fn border_top(color: bool, p: &Palette, version: &str) -> String {
    // Display: ╭─ Codineer vX.Y.Z ──────...──╮
    // Visible chars between ╭ and ╮: "─ Codineer vX.Y.Z " + bar
    let title_visible_len = 13 + version.len();
    let bar_len = BANNER_INNER_WIDTH.saturating_sub(title_visible_len);
    if color {
        format!(
            "{v}╭─ Codineer{r} {g}v{ver}{r} {v}{bar}╮{r}",
            v = p.violet,
            g = p.gray,
            r = p.r,
            ver = version,
            bar = "─".repeat(bar_len),
        )
    } else {
        format!("+- Codineer v{ver} {}+", "-".repeat(bar_len), ver = version,)
    }
}

fn border_bottom(color: bool, p: &Palette) -> String {
    if color {
        format!("{}╰{}╯{}", p.violet, "─".repeat(BANNER_INNER_WIDTH), p.r)
    } else {
        format!("+{}+", "-".repeat(BANNER_INNER_WIDTH))
    }
}

fn border_row(color: bool, p: &Palette, inner: &str) -> String {
    if color {
        format!("{v}│{r}{inner}{v}│{r}", v = p.violet, r = p.r)
    } else {
        format!("|{inner}|")
    }
}

fn left_column(color: bool, p: &Palette, ctx: &BannerContext<'_>) -> Vec<String> {
    let welcome = if color {
        format!("{}Welcome back · Codineer{}", p.bold_white, p.r)
    } else {
        "Welcome back · Codineer".to_string()
    };

    let logo: Vec<String> = if color {
        vec![
            format!("{}    ▄██▄{}", p.violet, p.r),
            format!("{} ▄██▀  ▀██▄{}", p.violet, p.r),
            format!("{}██  {}❯{}     ██{}", p.violet, p.cyan_fg, p.violet, p.r),
            format!("{}██     {}▍{}  ██{}", p.violet, p.amber, p.violet, p.r),
            format!("{} ▀██▄  ▄██▀{}", p.violet, p.r),
            format!("{}    ▀██▀{}", p.violet, p.r),
        ]
    } else {
        vec![
            "    ▄██▄".to_string(),
            " ▄██▀  ▀██▄".to_string(),
            "██  ❯     ██".to_string(),
            "██     ▍  ██".to_string(),
            " ▀██▄  ▄██▀".to_string(),
            "    ▀██▀".to_string(),
        ]
    };

    let model_line = format!("{} · {}", ctx.model, ctx.permissions);
    let model_styled = if color {
        format!("{}{}{}", p.dim, model_line, p.r)
    } else {
        model_line
    };

    let cwd_truncated = truncate_display(ctx.cwd_display, LEFT_COL - 4);
    let cwd_styled = if color {
        format!("{}{}{}", p.dim, cwd_truncated, p.r)
    } else {
        cwd_truncated
    };

    let mut lines = Vec::new();
    lines.push(String::new());
    lines.push(center_in(&welcome, LEFT_COL));
    lines.push(String::new());
    for l in &logo {
        let padded = fit_display_width(l, LOGO_WIDTH);
        lines.push(center_in(&padded, LEFT_COL));
    }
    lines.push(String::new());
    lines.push(center_in(&model_styled, LEFT_COL));
    lines.push(center_in(&cwd_styled, LEFT_COL));
    lines
}

fn right_column(color: bool, p: &Palette, ctx: &BannerContext<'_>) -> Vec<String> {
    let header = |text: &str| -> String {
        if color {
            format!("{}{}{}", p.violet, text, p.r)
        } else {
            text.to_string()
        }
    };

    let tips: Vec<&str> = if ctx.has_codineer_md {
        vec!["/help · Tab completes slash", "/vim for modal edit"]
    } else {
        vec!["/init · /help · /status", "— then ask for a task"]
    };

    let resume = tilde_session_path(ctx.session_path);
    let resume_cmd = format!("codineer --resume {}", resume.display());
    let resume_display = truncate_display(&resume_cmd, RIGHT_COL - 1);

    let separator = if color {
        let w = RIGHT_COL.min(30);
        format!("{}{}{}", p.dim, "─".repeat(w), p.r)
    } else {
        "-".repeat(RIGHT_COL.min(30))
    };

    let sid = truncate_display(ctx.session_id, RIGHT_COL - 1);
    let ws = truncate_display(ctx.workspace_summary, RIGHT_COL - 1);

    let mut lines = Vec::new();
    lines.push(header("Tips for getting started"));
    for t in &tips {
        lines.push(format!(" {t}"));
    }
    lines.push(String::new());
    lines.push(separator);
    lines.push(header("Session"));
    lines.push(format!(" {sid}"));
    lines.push(format!(" {ws}"));
    lines.push(String::new());
    lines.push(header("Resume"));
    lines.push(format!(" {resume_display}"));
    lines
}

fn tilde_session_path(path: &Path) -> std::path::PathBuf {
    let Some(home) = runtime::home_dir() else {
        return path.to_path_buf();
    };
    if path.starts_with(&home) {
        let rest = path.strip_prefix(&home).unwrap_or(path);
        std::path::PathBuf::from("~").join(rest)
    } else {
        path.to_path_buf()
    }
}
