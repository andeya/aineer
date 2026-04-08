use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

#[cfg(test)]
use crossterm::cursor::{MoveToColumn, RestorePosition, SavePosition};
#[cfg(test)]
use crossterm::queue;
use crossterm::style::{Color, Stylize};
#[cfg(test)]
use crossterm::style::{Print, ResetColor, SetForegroundColor};
#[cfg(test)]
use crossterm::terminal::{Clear, ClearType};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme as SyntectTheme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

fn color_enabled() -> bool {
    crate::style::color_for_stdout()
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
    name: &'static str,
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
            name: "dark",
        }
    }
}

#[cfg(test)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Spinner {
    frame_index: usize,
}

#[cfg(test)]
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ListKind {
    Unordered,
    Ordered { next_index: u64 },
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct TableState {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
    in_head: bool,
}

impl TableState {
    fn push_cell(&mut self) {
        let cell = self.current_cell.trim().to_string();
        self.current_row.push(cell);
        self.current_cell.clear();
    }

    fn finish_row(&mut self) {
        if self.current_row.is_empty() {
            return;
        }
        let row = std::mem::take(&mut self.current_row);
        if self.in_head {
            self.headers = row;
        } else {
            self.rows.push(row);
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct RenderState {
    emphasis: usize,
    strong: usize,
    heading_level: Option<u8>,
    quote: usize,
    list_stack: Vec<ListKind>,
    link_stack: Vec<LinkState>,
    table: Option<TableState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkState {
    destination: String,
    text: String,
}

impl RenderState {
    fn style_text(&self, text: &str, theme: &ColorTheme) -> String {
        if !color_enabled() {
            return text.to_string();
        }

        let mut style = text.stylize();

        if matches!(self.heading_level, Some(1 | 2)) || self.strong > 0 {
            style = style.bold();
        }
        if self.emphasis > 0 {
            style = style.italic();
        }

        if let Some(level) = self.heading_level {
            style = match level {
                1 => style.with(theme.heading),
                2 => style.white(),
                3 => style.with(Color::Blue),
                _ => style.with(Color::Grey),
            };
        } else if self.strong > 0 {
            style = style.with(theme.strong);
        } else if self.emphasis > 0 {
            style = style.with(theme.emphasis);
        }

        if self.quote > 0 {
            style = style.with(theme.quote);
        }

        format!("{style}")
    }

    fn append_raw(&mut self, output: &mut String, text: &str) {
        if let Some(link) = self.link_stack.last_mut() {
            link.text.push_str(text);
        } else if let Some(table) = self.table.as_mut() {
            table.current_cell.push_str(text);
        } else {
            output.push_str(text);
        }
    }

    fn append_styled(&mut self, output: &mut String, text: &str, theme: &ColorTheme) {
        let styled = self.style_text(text, theme);
        self.append_raw(output, &styled);
    }
}

#[derive(Debug)]
pub struct TerminalRenderer {
    syntax_set: SyntaxSet,
    syntax_theme: SyntectTheme,
    color_theme: ColorTheme,
}

impl Default for TerminalRenderer {
    fn default() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax_theme = ThemeSet::load_defaults()
            .themes
            .remove("base16-ocean.dark")
            .unwrap_or_default();
        Self {
            syntax_set,
            syntax_theme,
            color_theme: ColorTheme::default(),
        }
    }
}

impl TerminalRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    #[must_use]
    pub fn color_theme(&self) -> &ColorTheme {
        &self.color_theme
    }

    #[must_use]
    pub fn render_markdown(&self, markdown: &str) -> String {
        let mut output = String::new();
        let mut state = RenderState::default();
        let mut code_language = String::new();
        let mut code_buffer = String::new();
        let mut in_code_block = false;

        for event in Parser::new_ext(markdown, Options::all()) {
            self.render_event(
                event,
                &mut state,
                &mut output,
                &mut code_buffer,
                &mut code_language,
                &mut in_code_block,
            );
        }

        output.trim_end().to_string()
    }

    fn render_cmark_heading_start(
        state: &mut RenderState,
        level: HeadingLevel,
        output: &mut String,
    ) {
        Self::start_heading(state, level as u8, output);
    }

    fn render_cmark_paragraph_end(output: &mut String) {
        output.push_str("\n\n");
    }

    fn render_cmark_block_quote_start(&self, state: &mut RenderState, output: &mut String) {
        self.start_quote(state, output);
    }

    fn render_cmark_block_quote_end(state: &mut RenderState, output: &mut String) {
        state.quote = state.quote.saturating_sub(1);
        output.push('\n');
    }

    fn render_cmark_heading_end(state: &mut RenderState, output: &mut String) {
        state.heading_level = None;
        Self::render_cmark_paragraph_end(output);
    }

    fn render_cmark_line_break(state: &mut RenderState, output: &mut String) {
        state.append_raw(output, "\n");
    }

    fn render_cmark_list_start(state: &mut RenderState, first_item: Option<u64>) {
        let kind = match first_item {
            Some(index) => ListKind::Ordered { next_index: index },
            None => ListKind::Unordered,
        };
        state.list_stack.push(kind);
    }

    fn render_cmark_list_end(state: &mut RenderState, output: &mut String) {
        state.list_stack.pop();
        output.push('\n');
    }

    fn render_cmark_item_start(state: &mut RenderState, output: &mut String) {
        Self::start_item(state, output);
    }

    fn render_cmark_code_block_start(
        &self,
        kind: CodeBlockKind<'_>,
        output: &mut String,
        code_buffer: &mut String,
        code_language: &mut String,
        in_code_block: &mut bool,
    ) {
        *in_code_block = true;
        *code_language = match kind {
            CodeBlockKind::Indented => String::from("text"),
            CodeBlockKind::Fenced(lang) => lang.to_string(),
        };
        code_buffer.clear();
        self.start_code_block(code_language, output);
    }

    fn render_cmark_code_block_end(
        &self,
        output: &mut String,
        code_buffer: &mut String,
        code_language: &mut String,
        in_code_block: &mut bool,
    ) {
        self.finish_code_block(code_buffer, code_language, output);
        *in_code_block = false;
        code_language.clear();
        code_buffer.clear();
    }

    fn render_cmark_emphasis_start(state: &mut RenderState) {
        state.emphasis += 1;
    }

    fn render_cmark_emphasis_end(state: &mut RenderState) {
        state.emphasis = state.emphasis.saturating_sub(1);
    }

    fn render_cmark_strong_start(state: &mut RenderState) {
        state.strong += 1;
    }

    fn render_cmark_strong_end(state: &mut RenderState) {
        state.strong = state.strong.saturating_sub(1);
    }

    fn render_cmark_inline_code(
        &self,
        code: &impl AsRef<str>,
        state: &mut RenderState,
        output: &mut String,
    ) {
        let rendered = styled(
            &format!("`{}`", code.as_ref()),
            self.color_theme.inline_code,
        );
        state.append_raw(output, &rendered);
    }

    fn render_cmark_rule(output: &mut String) {
        output.push_str("---\n");
    }

    fn render_cmark_text(
        &self,
        text: &impl AsRef<str>,
        state: &mut RenderState,
        output: &mut String,
        code_buffer: &mut String,
        in_code_block: bool,
    ) {
        self.push_text(text.as_ref(), state, output, code_buffer, in_code_block);
    }

    fn render_cmark_html(html: &impl AsRef<str>, state: &mut RenderState, output: &mut String) {
        let sanitized: String = html
            .as_ref()
            .chars()
            .filter(|c| !c.is_control() || *c == '\n')
            .collect();
        state.append_raw(output, &sanitized);
    }

    fn render_cmark_footnote_ref(
        reference: &impl AsRef<str>,
        state: &mut RenderState,
        output: &mut String,
    ) {
        state.append_raw(output, &format!("[{}]", reference.as_ref()));
    }

    fn render_cmark_task_marker(done: bool, state: &mut RenderState, output: &mut String) {
        state.append_raw(output, if done { "[x] " } else { "[ ] " });
    }

    fn render_cmark_math(math: &impl AsRef<str>, state: &mut RenderState, output: &mut String) {
        state.append_raw(output, math.as_ref());
    }

    fn render_cmark_link_start(dest_url: &impl ToString, state: &mut RenderState) {
        state.link_stack.push(LinkState {
            destination: dest_url.to_string(),
            text: String::new(),
        });
    }

    fn render_cmark_link_end(&self, state: &mut RenderState, output: &mut String) {
        if let Some(link) = state.link_stack.pop() {
            let label = if link.text.is_empty() {
                link.destination.clone()
            } else {
                link.text
            };
            let rendered = styled_underlined(
                &format!("[{label}]({})", link.destination),
                self.color_theme.link,
            );
            state.append_raw(output, &rendered);
        }
    }

    fn render_cmark_image_start(
        &self,
        dest_url: &impl AsRef<str>,
        state: &mut RenderState,
        output: &mut String,
    ) {
        let rendered = styled(
            &format!("[image:{}]", dest_url.as_ref()),
            self.color_theme.link,
        );
        state.append_raw(output, &rendered);
    }

    fn render_cmark_table_start(state: &mut RenderState) {
        state.table = Some(TableState::default());
    }

    fn render_cmark_table_end(&self, state: &mut RenderState, output: &mut String) {
        if let Some(table) = state.table.take() {
            output.push_str(&self.render_table(&table));
            Self::render_cmark_paragraph_end(output);
        }
    }

    fn render_cmark_table_head_start(state: &mut RenderState) {
        if let Some(table) = state.table.as_mut() {
            table.in_head = true;
        }
    }

    fn render_cmark_table_head_end(state: &mut RenderState) {
        if let Some(table) = state.table.as_mut() {
            table.finish_row();
            table.in_head = false;
        }
    }

    fn render_cmark_table_row_start(state: &mut RenderState) {
        if let Some(table) = state.table.as_mut() {
            table.current_row.clear();
            table.current_cell.clear();
        }
    }

    fn render_cmark_table_row_end(state: &mut RenderState) {
        if let Some(table) = state.table.as_mut() {
            table.finish_row();
        }
    }

    fn render_cmark_table_cell_start(state: &mut RenderState) {
        if let Some(table) = state.table.as_mut() {
            table.current_cell.clear();
        }
    }

    fn render_cmark_table_cell_end(state: &mut RenderState) {
        if let Some(table) = state.table.as_mut() {
            table.push_cell();
        }
    }

    fn render_cmark_ignored() {}

    fn render_event(
        &self,
        event: Event<'_>,
        state: &mut RenderState,
        output: &mut String,
        code_buffer: &mut String,
        code_language: &mut String,
        in_code_block: &mut bool,
    ) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                Self::render_cmark_heading_start(state, level, output);
            }
            Event::End(TagEnd::Paragraph) => Self::render_cmark_paragraph_end(output),
            Event::Start(Tag::BlockQuote(..)) => self.render_cmark_block_quote_start(state, output),
            Event::End(TagEnd::BlockQuote(..)) => {
                Self::render_cmark_block_quote_end(state, output);
            }
            Event::End(TagEnd::Heading(..)) => Self::render_cmark_heading_end(state, output),
            Event::End(TagEnd::Item) | Event::SoftBreak | Event::HardBreak => {
                Self::render_cmark_line_break(state, output);
            }
            Event::Start(Tag::List(first_item)) => Self::render_cmark_list_start(state, first_item),
            Event::End(TagEnd::List(..)) => Self::render_cmark_list_end(state, output),
            Event::Start(Tag::Item) => Self::render_cmark_item_start(state, output),
            Event::Start(Tag::CodeBlock(kind)) => self.render_cmark_code_block_start(
                kind,
                output,
                code_buffer,
                code_language,
                in_code_block,
            ),
            Event::End(TagEnd::CodeBlock) => {
                self.render_cmark_code_block_end(output, code_buffer, code_language, in_code_block)
            }
            Event::Start(Tag::Emphasis) => Self::render_cmark_emphasis_start(state),
            Event::End(TagEnd::Emphasis) => Self::render_cmark_emphasis_end(state),
            Event::Start(Tag::Strong) => Self::render_cmark_strong_start(state),
            Event::End(TagEnd::Strong) => Self::render_cmark_strong_end(state),
            Event::Code(code) => self.render_cmark_inline_code(&code, state, output),
            Event::Rule => Self::render_cmark_rule(output),
            Event::Text(text) => {
                self.render_cmark_text(&text, state, output, code_buffer, *in_code_block);
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                Self::render_cmark_html(&html, state, output);
            }
            Event::FootnoteReference(reference) => {
                Self::render_cmark_footnote_ref(&reference, state, output);
            }
            Event::TaskListMarker(done) => Self::render_cmark_task_marker(done, state, output),
            Event::InlineMath(math) | Event::DisplayMath(math) => {
                Self::render_cmark_math(&math, state, output);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                Self::render_cmark_link_start(&dest_url, state);
            }
            Event::End(TagEnd::Link) => self.render_cmark_link_end(state, output),
            Event::Start(Tag::Image { dest_url, .. }) => {
                self.render_cmark_image_start(&dest_url, state, output);
            }
            Event::Start(Tag::Table(..)) => Self::render_cmark_table_start(state),
            Event::End(TagEnd::Table) => self.render_cmark_table_end(state, output),
            Event::Start(Tag::TableHead) => Self::render_cmark_table_head_start(state),
            Event::End(TagEnd::TableHead) => Self::render_cmark_table_head_end(state),
            Event::Start(Tag::TableRow) => Self::render_cmark_table_row_start(state),
            Event::End(TagEnd::TableRow) => Self::render_cmark_table_row_end(state),
            Event::Start(Tag::TableCell) => Self::render_cmark_table_cell_start(state),
            Event::End(TagEnd::TableCell) => Self::render_cmark_table_cell_end(state),
            Event::Start(Tag::Paragraph | Tag::MetadataBlock(..) | _)
            | Event::End(TagEnd::Image | TagEnd::MetadataBlock(..) | _) => {
                Self::render_cmark_ignored();
            }
        }
    }

    fn start_heading(state: &mut RenderState, level: u8, output: &mut String) {
        state.heading_level = Some(level);
        if !output.is_empty() {
            output.push('\n');
        }
    }

    fn start_quote(&self, state: &mut RenderState, output: &mut String) {
        state.quote += 1;
        let _ = write!(output, "{}", styled("│ ", self.color_theme.quote));
    }

    fn start_item(state: &mut RenderState, output: &mut String) {
        let depth = state.list_stack.len().saturating_sub(1);
        output.push_str(&"  ".repeat(depth));

        let marker = match state.list_stack.last_mut() {
            Some(ListKind::Ordered { next_index }) => {
                let value = *next_index;
                *next_index += 1;
                format!("{value}. ")
            }
            _ => "• ".to_string(),
        };
        output.push_str(&marker);
    }

    fn start_code_block(&self, code_language: &str, output: &mut String) {
        let label = if code_language.is_empty() {
            "code".to_string()
        } else {
            code_language.to_string()
        };
        let _ = writeln!(
            output,
            "{}",
            styled_bold(&format!("╭─ {label}"), self.color_theme.code_block_border)
        );
    }

    fn finish_code_block(&self, code_buffer: &str, code_language: &str, output: &mut String) {
        output.push_str(&self.highlight_code(code_buffer, code_language));
        let _ = write!(
            output,
            "{}",
            styled_bold("╰─", self.color_theme.code_block_border)
        );
        output.push_str("\n\n");
    }

    fn push_text(
        &self,
        text: &str,
        state: &mut RenderState,
        output: &mut String,
        code_buffer: &mut String,
        in_code_block: bool,
    ) {
        if in_code_block {
            code_buffer.push_str(text);
        } else {
            state.append_styled(output, text, &self.color_theme);
        }
    }

    fn render_table(&self, table: &TableState) -> String {
        let mut rows = Vec::new();
        if !table.headers.is_empty() {
            rows.push(table.headers.clone());
        }
        rows.extend(table.rows.iter().cloned());

        if rows.is_empty() {
            return String::new();
        }

        let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
        let widths = (0..column_count)
            .map(|column| {
                rows.iter()
                    .filter_map(|row| row.get(column))
                    .map(|cell| visible_width(cell))
                    .max()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();

        let border = styled("│", self.color_theme.table_border);
        let separator = widths
            .iter()
            .map(|width| "─".repeat(*width + 2))
            .collect::<Vec<_>>()
            .join(&styled("┼", self.color_theme.table_border));
        let separator = format!("{border}{separator}{border}");

        let mut output = String::new();
        if !table.headers.is_empty() {
            output.push_str(&self.render_table_row(&table.headers, &widths, true));
            output.push('\n');
            output.push_str(&separator);
            if !table.rows.is_empty() {
                output.push('\n');
            }
        }

        for (index, row) in table.rows.iter().enumerate() {
            output.push_str(&self.render_table_row(row, &widths, false));
            if index + 1 < table.rows.len() {
                output.push('\n');
            }
        }

        output
    }

    fn render_table_row(&self, row: &[String], widths: &[usize], is_header: bool) -> String {
        let border = styled("│", self.color_theme.table_border);
        let mut line = String::new();
        line.push_str(&border);

        for (index, width) in widths.iter().enumerate() {
            let cell = row.get(index).map_or("", String::as_str);
            line.push(' ');
            if is_header {
                let _ = write!(line, "{}", styled_bold(cell, self.color_theme.heading));
            } else {
                line.push_str(cell);
            }
            let padding = width.saturating_sub(visible_width(cell));
            line.push_str(&" ".repeat(padding + 1));
            line.push_str(&border);
        }

        line
    }

    #[must_use]
    pub fn highlight_code(&self, code: &str, language: &str) -> String {
        if !color_enabled() {
            return code.to_string();
        }

        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let mut syntax_highlighter = HighlightLines::new(syntax, &self.syntax_theme);
        let mut colored_output = String::new();

        for line in LinesWithEndings::from(code) {
            match syntax_highlighter.highlight_line(line, &self.syntax_set) {
                Ok(ranges) => {
                    let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                    colored_output.push_str(&apply_code_block_background(&escaped));
                }
                Err(_) => colored_output.push_str(&apply_code_block_background(line)),
            }
        }

        colored_output
    }

    pub fn stream_markdown(&self, markdown: &str, out: &mut impl Write) -> io::Result<()> {
        let rendered_markdown = self.render_markdown(markdown);
        write!(out, "{rendered_markdown}")?;
        if !rendered_markdown.ends_with('\n') {
            writeln!(out)?;
        }
        out.flush()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MarkdownStreamState {
    pending: String,
}

impl MarkdownStreamState {
    #[must_use]
    pub fn push(&mut self, renderer: &TerminalRenderer, delta: &str) -> Option<String> {
        self.pending.push_str(delta);
        let split = find_stream_safe_boundary(&self.pending)?;
        let ready = self.pending[..split].to_string();
        self.pending.drain(..split);
        Some(renderer.render_markdown(&ready))
    }

    #[must_use]
    pub fn flush(&mut self, renderer: &TerminalRenderer) -> Option<String> {
        if self.pending.trim().is_empty() {
            self.pending.clear();
            None
        } else {
            let pending = std::mem::take(&mut self.pending);
            Some(renderer.render_markdown(&pending))
        }
    }
}

/// Writer wrapper that prepends a gutter prefix (`  ⎿  `) to each output line,
/// matching Claude Code's `MessageResponse` layout.
pub(crate) struct GutterWriter<W> {
    inner: W,
    first_prefix: Vec<u8>,
    cont_prefix: Vec<u8>,
    at_line_start: bool,
    first_line: bool,
}

impl<W: Write> GutterWriter<W> {
    pub(crate) fn new(inner: W, first_prefix: String, cont_prefix: String) -> Self {
        Self {
            inner,
            first_prefix: first_prefix.into_bytes(),
            cont_prefix: cont_prefix.into_bytes(),
            at_line_start: true,
            first_line: true,
        }
    }
}

impl<W: Write> Write for GutterWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for &byte in buf {
            if self.at_line_start && byte != b'\n' {
                let prefix = if self.first_line {
                    self.first_line = false;
                    &self.first_prefix
                } else {
                    &self.cont_prefix
                };
                self.inner.write_all(prefix)?;
                self.at_line_start = false;
            }
            self.inner.write_all(std::slice::from_ref(&byte))?;
            if byte == b'\n' {
                self.at_line_start = true;
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub(crate) fn gutter_prefixes() -> (String, String) {
    let p = crate::style::Palette::new(crate::style::color_for_stdout());
    let first = if p.dim.is_empty() {
        "  ⎿  ".to_string()
    } else {
        format!("{}  ⎿  {}", p.dim, p.r)
    };
    let cont = "     ".to_string();
    (first, cont)
}

fn apply_code_block_background(line: &str) -> String {
    let trimmed = line.trim_end_matches('\n');
    let trailing_newline = if trimmed.len() == line.len() {
        ""
    } else {
        "\n"
    };
    let with_background = trimmed.replace("\u{1b}[0m", "\u{1b}[0;48;5;236m");
    format!("\u{1b}[48;5;236m{with_background}\u{1b}[0m{trailing_newline}")
}

fn find_stream_safe_boundary(markdown: &str) -> Option<usize> {
    let mut in_fence = false;
    let mut last_boundary = None;

    for (offset, line) in markdown.split_inclusive('\n').scan(0usize, |cursor, line| {
        let start = *cursor;
        *cursor += line.len();
        Some((start, line))
    }) {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            if !in_fence {
                last_boundary = Some(offset + line.len());
            }
            continue;
        }

        if in_fence {
            continue;
        }

        if trimmed.is_empty() {
            last_boundary = Some(offset + line.len());
        }
    }

    last_boundary
}

fn visible_width(input: &str) -> usize {
    strip_ansi(input).chars().count()
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            output.push(ch);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{color_enabled, strip_ansi, MarkdownStreamState, Spinner, TerminalRenderer};

    #[test]
    fn renders_markdown_with_styling_and_lists() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output = terminal_renderer
            .render_markdown("# Heading\n\nThis is **bold** and *italic*.\n\n- item\n\n`code`");

        assert!(markdown_output.contains("Heading"));
        assert!(markdown_output.contains("• item"));
        assert!(markdown_output.contains("code"));
        if color_enabled() {
            assert!(markdown_output.contains('\u{1b}'));
        }
    }

    #[test]
    fn renders_links_as_colored_markdown_labels() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output =
            terminal_renderer.render_markdown("See [Codineer](https://example.com/docs) now.");
        let plain_text = strip_ansi(&markdown_output);

        assert!(plain_text.contains("[Codineer](https://example.com/docs)"));
        if color_enabled() {
            assert!(markdown_output.contains('\u{1b}'));
        }
    }

    #[test]
    fn highlights_fenced_code_blocks() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output =
            terminal_renderer.render_markdown("```rust\nfn hi() { println!(\"hi\"); }\n```");
        let plain_text = strip_ansi(&markdown_output);

        assert!(plain_text.contains("╭─ rust"));
        assert!(plain_text.contains("fn hi"));
        if color_enabled() {
            assert!(markdown_output.contains('\u{1b}'));
            assert!(markdown_output.contains("[48;5;236m"));
        }
    }

    #[test]
    fn renders_ordered_and_nested_lists() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output =
            terminal_renderer.render_markdown("1. first\n2. second\n   - nested\n   - child");
        let plain_text = strip_ansi(&markdown_output);

        assert!(plain_text.contains("1. first"));
        assert!(plain_text.contains("2. second"));
        assert!(plain_text.contains("  • nested"));
        assert!(plain_text.contains("  • child"));
    }

    #[test]
    fn renders_tables_with_alignment() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output = terminal_renderer
            .render_markdown("| Name | Value |\n| ---- | ----- |\n| alpha | 1 |\n| beta | 22 |");
        let plain_text = strip_ansi(&markdown_output);
        let lines = plain_text.lines().collect::<Vec<_>>();

        assert_eq!(lines[0], "│ Name  │ Value │");
        assert_eq!(lines[1], "│───────┼───────│");
        assert_eq!(lines[2], "│ alpha │ 1     │");
        assert_eq!(lines[3], "│ beta  │ 22    │");
        if color_enabled() {
            assert!(markdown_output.contains('\u{1b}'));
        }
    }

    #[test]
    fn streaming_state_waits_for_complete_blocks() {
        let renderer = TerminalRenderer::new();
        let mut state = MarkdownStreamState::default();

        assert_eq!(state.push(&renderer, "# Heading"), None);
        let flushed = state
            .push(&renderer, "\n\nParagraph\n\n")
            .expect("completed block");
        let plain_text = strip_ansi(&flushed);
        assert!(plain_text.contains("Heading"));
        assert!(plain_text.contains("Paragraph"));

        assert_eq!(state.push(&renderer, "```rust\nfn main() {}\n"), None);
        let code = state
            .push(&renderer, "```\n")
            .expect("closed code fence flushes");
        assert!(strip_ansi(&code).contains("fn main()"));
    }

    #[test]
    fn spinner_advances_frames() {
        let terminal_renderer = TerminalRenderer::new();
        let mut spinner = Spinner::new();
        let mut out = Vec::new();
        spinner
            .tick("Working", terminal_renderer.color_theme(), &mut out)
            .expect("tick succeeds");
        spinner
            .tick("Working", terminal_renderer.color_theme(), &mut out)
            .expect("tick succeeds");

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("Working"));
    }
}
