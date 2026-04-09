use alacritty_terminal::index::Point as TerminalGridPoint;
use alacritty_terminal::term::cell;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{Color, NamedColor};
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Shape, Stroke, Vec2};

use crate::backend::RenderableContent;
use crate::theme::TerminalTheme;

struct TextRun {
    line: i32,
    next_column: usize,
    x: f32,
    y: f32,
    fg: Color32,
    text: String,
}

struct BackgroundRun {
    line: i32,
    next_column: usize,
    rect: Rect,
    bg: Color32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GridCacheKey {
    rect_min_x: u32,
    rect_min_y: u32,
    rect_w: u32,
    rect_h: u32,
    cell_w: u32,
    cell_h: u32,
    display_offset: usize,
    generation: u64,
}

impl GridCacheKey {
    fn new(rect: Rect, cw: f32, ch: f32, offset: usize, gen: u64) -> Self {
        Self {
            rect_min_x: rect.min.x.to_bits(),
            rect_min_y: rect.min.y.to_bits(),
            rect_w: rect.width().to_bits(),
            rect_h: rect.height().to_bits(),
            cell_w: cw.to_bits(),
            cell_h: ch.to_bits(),
            display_offset: offset,
            generation: gen,
        }
    }
}

#[derive(Clone, Default)]
pub struct TerminalGridCache {
    key: Option<GridCacheKey>,
    shapes: Vec<Shape>,
}

/// Render terminal grid with text-run batching, background-run batching,
/// bg/fg layer separation, and frame-to-frame caching.
#[allow(clippy::too_many_arguments)]
pub fn render_grid(
    painter: &egui::Painter,
    rect: Rect,
    content: &RenderableContent,
    theme: &TerminalTheme,
    font_id: FontId,
    mouse_grid_pos: TerminalGridPoint,
    grid_cache: &mut TerminalGridCache,
    allow_cache: bool,
) {
    let cw = content.terminal_size.cell_width as f32;
    let ch = content.terminal_size.cell_height as f32;
    let key = GridCacheKey::new(
        rect,
        cw,
        ch,
        content.grid.display_offset(),
        content.generation,
    );

    if allow_cache && grid_cache.key == Some(key) {
        painter.extend(grid_cache.shapes.iter().cloned());
        return;
    }

    let shapes = build_grid_shapes(
        painter,
        rect,
        content,
        theme,
        &font_id,
        cw,
        ch,
        mouse_grid_pos,
    );
    painter.extend(shapes.iter().cloned());
    grid_cache.key = Some(key);
    grid_cache.shapes = shapes;
}

#[allow(clippy::too_many_arguments)]
fn build_grid_shapes(
    painter: &egui::Painter,
    rect: Rect,
    content: &RenderableContent,
    theme: &TerminalTheme,
    font_id: &FontId,
    cw: f32,
    ch: f32,
    mouse_grid_pos: TerminalGridPoint,
) -> Vec<Shape> {
    let global_bg = theme.get_color(Color::Named(NamedColor::Background));
    let display_offset = content.grid.display_offset();
    let is_app_cursor = content.terminal_mode.contains(TermMode::APP_CURSOR);

    let mut bg_shapes = vec![Shape::rect_filled(rect, CornerRadius::ZERO, global_bg)];
    let mut fg_shapes: Vec<Shape> = Vec::new();

    painter.fonts_mut(|fonts| {
        let mut text_run: Option<TextRun> = None;
        let mut bg_run: Option<BackgroundRun> = None;

        for indexed in content.grid.display_iter() {
            let flags = indexed.cell.flags;

            if flags.contains(cell::Flags::WIDE_CHAR_SPACER) {
                flush_bg_run(&mut bg_shapes, &mut bg_run);
                continue;
            }

            let is_wide = flags.contains(cell::Flags::WIDE_CHAR);
            let is_cursor = content.grid.cursor.point == indexed.point;

            let col = indexed.point.column.0;
            let line_num = indexed.point.line.0 + display_offset as i32;
            let x = rect.min.x + col as f32 * cw;
            let y = rect.min.y + line_num as f32 * ch;
            let w = if is_wide { cw * 2.0 } else { cw };

            let mut fg = theme.get_color(indexed.fg);
            let mut bg = theme.get_color(indexed.bg);

            if flags.intersects(cell::Flags::DIM | cell::Flags::DIM_BOLD) {
                fg = fg.linear_multiply(0.7);
            }

            let is_selected = content
                .selectable_range
                .is_some_and(|r| r.contains(indexed.point));

            if flags.contains(cell::Flags::INVERSE) || is_selected {
                std::mem::swap(&mut fg, &mut bg);
            }

            // --- Search match highlight ---
            let in_search_match = content
                .search_match
                .as_ref()
                .is_some_and(|m| m.contains(&indexed.point));
            if in_search_match {
                bg = theme.search_match_bg;
                fg = theme.search_match_fg;
            }

            // --- Background batching ---
            if bg != global_bg {
                append_bg_rect(
                    &mut bg_shapes,
                    &mut bg_run,
                    line_num,
                    col,
                    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w + 1.0, ch + 1.0)),
                    bg,
                );
            } else {
                flush_bg_run(&mut bg_shapes, &mut bg_run);
            }

            // --- Cursor ---
            if is_cursor {
                flush_bg_run(&mut bg_shapes, &mut bg_run);
                let cursor_color = theme.get_color(content.cursor.fg);
                bg_shapes.push(Shape::rect_filled(
                    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, ch)),
                    CornerRadius::default(),
                    cursor_color,
                ));
                if is_app_cursor {
                    std::mem::swap(&mut fg, &mut bg);
                }
            }

            // --- Hyperlink underline ---
            if let Some(ref hovered) = content.hovered_hyperlink {
                if hovered.contains(&indexed.point) && hovered.contains(&mouse_grid_pos) {
                    fg_shapes.push(Shape::line_segment(
                        [Pos2::new(x, y + ch), Pos2::new(x + w, y + ch)],
                        Stroke::new(ch * 0.15, fg),
                    ));
                }
            }

            // --- Text batching ---
            let c = indexed.c;
            if is_batchable(flags, c) && !is_cursor {
                let can_continue = text_run
                    .as_ref()
                    .is_some_and(|r| r.line == line_num && r.next_column == col && r.fg == fg);
                if can_continue {
                    if let Some(r) = &mut text_run {
                        r.text.push(c);
                        r.next_column = col + 1;
                    }
                    continue;
                }
                flush_text_run(fonts, &mut fg_shapes, font_id, &mut text_run);
                if c != ' ' && c != '\t' {
                    let mut t = String::with_capacity(64);
                    t.push(c);
                    text_run = Some(TextRun {
                        line: line_num,
                        next_column: col + 1,
                        x,
                        y,
                        fg,
                        text: t,
                    });
                }
                continue;
            }

            // --- Non-batchable cell ---
            flush_text_run(fonts, &mut fg_shapes, font_id, &mut text_run);

            if c != ' ' && c != '\t' && !flags.contains(cell::Flags::HIDDEN) {
                fg_shapes.push(Shape::text(
                    fonts,
                    Pos2::new(x + w / 2.0, y),
                    Align2::CENTER_TOP,
                    c,
                    font_id.clone(),
                    fg,
                ));
            }

            if has_decoration(flags) {
                append_decoration(&mut fg_shapes, x, y, w, ch, fg, flags);
            }
        }

        flush_text_run(fonts, &mut fg_shapes, font_id, &mut text_run);
        flush_bg_run(&mut bg_shapes, &mut bg_run);
    });

    bg_shapes.extend(fg_shapes);
    bg_shapes
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_batchable(flags: cell::Flags, _c: char) -> bool {
    !flags.intersects(
        cell::Flags::WIDE_CHAR
            | cell::Flags::WIDE_CHAR_SPACER
            | cell::Flags::HIDDEN
            | cell::Flags::UNDERLINE
            | cell::Flags::DOUBLE_UNDERLINE
            | cell::Flags::STRIKEOUT,
    )
}

fn has_decoration(flags: cell::Flags) -> bool {
    flags
        .intersects(cell::Flags::UNDERLINE | cell::Flags::DOUBLE_UNDERLINE | cell::Flags::STRIKEOUT)
}

fn append_decoration(
    shapes: &mut Vec<Shape>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color32,
    flags: cell::Flags,
) {
    if flags.intersects(cell::Flags::UNDERLINE | cell::Flags::DOUBLE_UNDERLINE) {
        let uy = y + h - 1.5;
        shapes.push(Shape::line_segment(
            [Pos2::new(x, uy), Pos2::new(x + w, uy)],
            Stroke::new(1.0, color),
        ));
    }
    if flags.contains(cell::Flags::STRIKEOUT) {
        let sy = y + h / 2.0;
        shapes.push(Shape::line_segment(
            [Pos2::new(x, sy), Pos2::new(x + w, sy)],
            Stroke::new(1.0, color),
        ));
    }
}

fn flush_text_run(
    fonts: &mut egui::epaint::text::FontsView<'_>,
    shapes: &mut Vec<Shape>,
    font_id: &FontId,
    run: &mut Option<TextRun>,
) {
    let Some(r) = run.take() else { return };
    if r.text.is_empty() {
        return;
    }
    shapes.push(Shape::text(
        fonts,
        Pos2::new(r.x, r.y),
        Align2::LEFT_TOP,
        r.text,
        font_id.clone(),
        r.fg,
    ));
}

fn append_bg_rect(
    shapes: &mut Vec<Shape>,
    run: &mut Option<BackgroundRun>,
    line: i32,
    column: usize,
    cell_rect: Rect,
    bg: Color32,
) {
    let can_continue = run
        .as_ref()
        .is_some_and(|r| r.line == line && r.next_column == column && r.bg == bg);
    if can_continue {
        if let Some(r) = run {
            r.rect.max.x = cell_rect.max.x;
            r.next_column = column + 1;
        }
        return;
    }
    flush_bg_run(shapes, run);
    *run = Some(BackgroundRun {
        line,
        next_column: column + 1,
        rect: cell_rect,
        bg,
    });
}

fn flush_bg_run(shapes: &mut Vec<Shape>, run: &mut Option<BackgroundRun>) {
    let Some(r) = run.take() else { return };
    shapes.push(Shape::rect_filled(r.rect, CornerRadius::ZERO, r.bg));
}
