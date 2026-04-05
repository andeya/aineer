//! Welcome panel for the REPL (layout inspired by Claude Code: framed summary + resume hint).

use std::path::Path;

use crate::style::Palette;

const INNER_WIDTH: usize = 58;

pub(crate) struct BannerContext<'a> {
    pub workspace_summary: &'a str,
    pub cwd_display: &'a str,
    pub model: &'a str,
    pub permissions: &'a str,
    pub session_id: &'a str,
    pub session_path: &'a Path,
    pub has_codineer_md: bool,
}

fn fit_line(text: &str) -> String {
    let mut t = text.to_string();
    if t.chars().count() > INNER_WIDTH {
        t = t
            .chars()
            .take(INNER_WIDTH.saturating_sub(1))
            .chain("…".chars())
            .collect();
    }
    let pad = INNER_WIDTH.saturating_sub(t.chars().count());
    format!("{t}{}", " ".repeat(pad))
}

fn framed_row(color: bool, p: &Palette, text: &str) -> String {
    let inner = fit_line(text);
    if color {
        let b = p.amber;
        let r = p.r;
        format!("{b}│{r}{inner}{b}│{r}")
    } else {
        format!("|{inner}|")
    }
}

fn framed_top_bottom(color: bool, p: &Palette) -> (String, String) {
    if color {
        let b = p.amber;
        let r = p.r;
        let bar = format!("{b}{}{r}", "─".repeat(INNER_WIDTH));
        (format!("{b}╭{bar}╮{r}"), format!("{b}╰{bar}╯{r}"))
    } else {
        let bar = "-".repeat(INNER_WIDTH);
        (format!("+{bar}+"), format!("+{bar}+"))
    }
}

pub(crate) fn welcome_banner(color: bool, ctx: BannerContext<'_>) -> String {
    let p = Palette::new(color);
    let (top, bot) = framed_top_bottom(color, &p);

    let title = if color {
        let bw = p.bold_white;
        let r = p.r;
        format!("  {bw}Welcome back{r} · Codineer")
    } else {
        "  Welcome back · Codineer".to_string()
    };

    let tip = if ctx.has_codineer_md {
        "Tips   /help · Tab completes slash commands · /vim for modal edit"
    } else {
        "Tips   /init · /help · /status — then ask for a task"
    };

    let resume = tilde_session_path(ctx.session_path);
    let resume_cmd = format!("codineer --resume {}", resume.display());

    [
        top,
        framed_row(color, &p, &title),
        framed_row(color, &p, ""),
        framed_row(
            color,
            &p,
            &format!("  Model      {}", ctx.model),
        ),
        framed_row(
            color,
            &p,
            &format!("  Directory  {}", ctx.cwd_display),
        ),
        framed_row(
            color,
            &p,
            &format!(
                "  Workspace  {} · {}",
                ctx.workspace_summary, ctx.permissions
            ),
        ),
        framed_row(
            color,
            &p,
            &format!("  Session    {}", ctx.session_id),
        ),
        framed_row(color, &p, ""),
        framed_row(color, &p, tip),
        framed_row(color, &p, ""),
        framed_row(color, &p, &format!("  Resume     {resume_cmd}")),
        bot,
    ]
    .join("\n")
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
