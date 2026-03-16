use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

use crossterm::cursor::{MoveToColumn, RestorePosition, SavePosition};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor, Stylize};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{execute, queue};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

fn color_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if let Some(val) = std::env::var_os("CLICOLOR") {
        return val != "0";
    }
    true
}

fn styled(text: &str, color: Color) -> String {
    if color_enabled() {
        format!("{}", text.with(color))
    } else {
        text.to_string()
    }
}

fn styled_bold(text: &str, color: Color) -> String {
    if color_enabled() {
        format!("{}", text.bold().with(color))
    } else {
        text.to_string()
    }
}

fn styled_underlined(text: &str, color: Color) -> String {
    if color_enabled() {
        format!("{}", text.underlined().with(color))
    } else {
        text.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorTheme {
    heading: Color,
    emphasis: Color,
    strong: Color,
    inline_code: Color,
    link: Color,
    quote: Color,
