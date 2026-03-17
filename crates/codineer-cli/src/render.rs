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
    table_border: Color,
    code_block_border: Color,
    spinner_active: Color,
    spinner_done: Color,
    spinner_failed: Color,
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self {
            heading: Color::Cyan,
            emphasis: Color::Magenta,
            strong: Color::Yellow,
            inline_code: Color::Green,
            link: Color::Blue,
            quote: Color::DarkGrey,
            table_border: Color::DarkCyan,
            code_block_border: Color::DarkGrey,
            spinner_active: Color::Blue,
            spinner_done: Color::Green,
            spinner_failed: Color::Red,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Spinner {
    frame_index: usize,
}

impl Spinner {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(
        &mut self,
        label: &str,
        theme: &ColorTheme,
        out: &mut impl Write,
    ) -> io::Result<()> {
        let frame = Self::FRAMES[self.frame_index % Self::FRAMES.len()];
        self.frame_index += 1;
        if color_enabled() {
            queue!(
                out,
                SavePosition,
                MoveToColumn(0),
                Clear(ClearType::CurrentLine),
                SetForegroundColor(theme.spinner_active),
                Print(format!("{frame} {label}")),
                ResetColor,
                RestorePosition
            )?;
        } else {
            queue!(
                out,
                SavePosition,
                MoveToColumn(0),
                Clear(ClearType::CurrentLine),
                Print(format!("{frame} {label}")),
                RestorePosition
            )?;
        }
        out.flush()
    }

    pub fn finish(
        &mut self,
        label: &str,
        theme: &ColorTheme,
        out: &mut impl Write,
    ) -> io::Result<()> {
        self.frame_index = 0;
        if color_enabled() {
            execute!(
                out,
                MoveToColumn(0),
                Clear(ClearType::CurrentLine),
                SetForegroundColor(theme.spinner_done),
                Print(format!("✔ {label}\n")),
                ResetColor
            )?;
        } else {
            execute!(
                out,
                MoveToColumn(0),
                Clear(ClearType::CurrentLine),
                Print(format!("✔ {label}\n"))
            )?;
        }
        out.flush()
    }

    pub fn fail(
        &mut self,
        label: &str,
        theme: &ColorTheme,
        out: &mut impl Write,
    ) -> io::Result<()> {
        self.frame_index = 0;
        if color_enabled() {
            execute!(
                out,
                MoveToColumn(0),
