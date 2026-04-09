use std::cell::Cell;

use egui::{Color32, CornerRadius, Margin, Shadow, Stroke, Style, Vec2, Visuals};

// ── Legacy layout constants (kept for compat) ───────────────────
pub const TOOLBAR_HEIGHT: f32 = 38.0;
pub const PANEL_PADDING: f32 = 8.0;
pub const CARD_CORNER_RADIUS: f32 = 10.0;
pub const CARD_INNER_MARGIN: f32 = 12.0;
pub const BUTTON_CORNER_RADIUS: f32 = 8.0;
pub const INPUT_CORNER_RADIUS: f32 = 8.0;

// ── Design tokens ────────────────────────────────────────────────

pub mod spacing {
    pub const XXXS: f32 = 1.0;
    pub const XXS: f32 = 2.0;
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 6.0;
    pub const MD: f32 = 8.0;
    pub const LG: f32 = 12.0;
    pub const XL: f32 = 16.0;
    pub const XXL: f32 = 24.0;
    pub const XXXL: f32 = 32.0;
}

pub mod font_size {
    pub const CAPTION: f32 = 10.0;
    pub const SMALL: f32 = 11.0;
    pub const BODY: f32 = 13.0;
    pub const SUBTITLE: f32 = 14.0;
    pub const TITLE: f32 = 16.0;
    pub const HEADING: f32 = 20.0;
}

pub mod radius {
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 6.0;
    pub const LG: f32 = 8.0;
    pub const XL: f32 = 12.0;
}

pub const STATUS_BAR_HEIGHT: f32 = 24.0;

// ── Theme mode ───────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub fn parse(s: &str) -> Self {
        match s {
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

// ── Palette ──────────────────────────────────────────────────────
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Color32,
    pub bg_elevated: Color32,
    pub panel_bg: Color32,
    pub panel_bg_alt: Color32,
    pub surface: Color32,

    pub fg: Color32,
    pub fg_soft: Color32,
    pub fg_dim: Color32,
    pub fg_muted: Color32,

    pub accent: Color32,
    pub accent_light: Color32,
    pub accent_cyan: Color32,
    pub amber: Color32,

    pub border_subtle: Color32,
    pub border_strong: Color32,

    pub success: Color32,
    pub error: Color32,
    pub warning: Color32,

    pub diff_add_bg: Color32,
    pub diff_add_fg: Color32,
    pub diff_del_bg: Color32,
    pub diff_del_fg: Color32,
    pub diff_hunk_header: Color32,

    pub shell_running_bg: Color32,
    pub shell_success_bg: Color32,
    pub shell_error_bg: Color32,
    pub chat_bg: Color32,
    pub system_bg: Color32,

    pub button_bg: Color32,
    pub button_hover: Color32,
    pub input_bg: Color32,
    pub tab_active_bg: Color32,
    pub tab_inactive_bg: Color32,
}

impl Palette {
    pub const DARK: Self = Self {
        bg: Color32::from_rgb(14, 14, 26),
        bg_elevated: Color32::from_rgb(20, 21, 33),
        panel_bg: Color32::from_rgb(24, 24, 36),
        panel_bg_alt: Color32::from_rgb(30, 30, 44),
        surface: Color32::from_rgb(36, 36, 52),

        fg: Color32::from_rgb(224, 228, 240),
        fg_soft: Color32::from_rgb(170, 178, 200),
        fg_dim: Color32::from_rgb(108, 116, 140),
        fg_muted: Color32::from_rgb(80, 86, 110),

        accent: Color32::from_rgb(99, 102, 241),
        accent_light: Color32::from_rgb(129, 140, 248),
        accent_cyan: Color32::from_rgb(6, 182, 212),
        amber: Color32::from_rgb(251, 191, 36),

        border_subtle: Color32::from_rgb(40, 42, 60),
        border_strong: Color32::from_rgb(60, 64, 90),

        success: Color32::from_rgb(78, 203, 113),
        error: Color32::from_rgb(243, 139, 168),
        warning: Color32::from_rgb(233, 190, 109),

        diff_add_bg: Color32::from_rgb(24, 50, 32),
        diff_add_fg: Color32::from_rgb(120, 220, 120),
        diff_del_bg: Color32::from_rgb(55, 24, 28),
        diff_del_fg: Color32::from_rgb(240, 130, 140),
        diff_hunk_header: Color32::from_rgb(100, 150, 220),

        shell_running_bg: Color32::from_rgb(28, 28, 42),
        shell_success_bg: Color32::from_rgb(22, 36, 28),
        shell_error_bg: Color32::from_rgb(38, 22, 26),
        chat_bg: Color32::from_rgb(24, 24, 42),
        system_bg: Color32::from_rgb(30, 28, 42),

        button_bg: Color32::from_rgb(36, 38, 54),
        button_hover: Color32::from_rgb(46, 48, 68),
        input_bg: Color32::from_rgb(20, 21, 33),
        tab_active_bg: Color32::from_rgb(42, 42, 58),
        tab_inactive_bg: Color32::from_rgb(24, 24, 36),
    };

    pub const LIGHT: Self = Self {
        bg: Color32::from_rgb(252, 252, 255),
        bg_elevated: Color32::from_rgb(245, 245, 250),
        panel_bg: Color32::from_rgb(240, 240, 246),
        panel_bg_alt: Color32::from_rgb(233, 233, 240),
        surface: Color32::from_rgb(224, 224, 234),

        fg: Color32::from_rgb(28, 32, 48),
        fg_soft: Color32::from_rgb(64, 72, 96),
        fg_dim: Color32::from_rgb(100, 108, 130),
        fg_muted: Color32::from_rgb(145, 150, 170),

        accent: Color32::from_rgb(79, 82, 210),
        accent_light: Color32::from_rgb(99, 102, 220),
        accent_cyan: Color32::from_rgb(0, 150, 180),
        amber: Color32::from_rgb(180, 130, 0),

        border_subtle: Color32::from_rgb(210, 212, 224),
        border_strong: Color32::from_rgb(180, 184, 200),

        success: Color32::from_rgb(30, 150, 60),
        error: Color32::from_rgb(200, 60, 80),
        warning: Color32::from_rgb(180, 130, 20),

        diff_add_bg: Color32::from_rgb(220, 245, 225),
        diff_add_fg: Color32::from_rgb(30, 140, 50),
        diff_del_bg: Color32::from_rgb(250, 220, 225),
        diff_del_fg: Color32::from_rgb(200, 50, 70),
        diff_hunk_header: Color32::from_rgb(60, 100, 180),

        shell_running_bg: Color32::from_rgb(240, 242, 248),
        shell_success_bg: Color32::from_rgb(228, 245, 232),
        shell_error_bg: Color32::from_rgb(248, 228, 232),
        chat_bg: Color32::from_rgb(235, 238, 250),
        system_bg: Color32::from_rgb(238, 236, 245),

        button_bg: Color32::from_rgb(230, 232, 240),
        button_hover: Color32::from_rgb(216, 218, 230),
        input_bg: Color32::from_rgb(248, 248, 252),
        tab_active_bg: Color32::from_rgb(255, 255, 255),
        tab_inactive_bg: Color32::from_rgb(238, 238, 244),
    };
}

// ── Thread-local active palette ──────────────────────────────────
thread_local! {
    static ACTIVE_PALETTE: Cell<Palette> = const { Cell::new(Palette::DARK) };
    static ACTIVE_MODE: Cell<ThemeMode> = const { Cell::new(ThemeMode::Dark) };
}

pub fn current_mode() -> ThemeMode {
    ACTIVE_MODE.with(|m| m.get())
}

#[inline]
fn pal() -> Palette {
    ACTIVE_PALETTE.with(|p| p.get())
}

// ── Color accessor functions ─────────────────────────────────────
macro_rules! color_fn {
    ($fn_name:ident, $field:ident) => {
        #[allow(non_snake_case)]
        #[inline]
        pub fn $fn_name() -> Color32 {
            pal().$field
        }
    };
}

color_fn!(BG, bg);
color_fn!(BG_ELEVATED, bg_elevated);
color_fn!(PANEL_BG, panel_bg);
color_fn!(PANEL_BG_ALT, panel_bg_alt);
color_fn!(SURFACE, surface);

color_fn!(FG, fg);
color_fn!(FG_SOFT, fg_soft);
color_fn!(FG_DIM, fg_dim);
color_fn!(FG_MUTED, fg_muted);

color_fn!(ACCENT, accent);
color_fn!(ACCENT_LIGHT, accent_light);
color_fn!(ACCENT_CYAN, accent_cyan);
color_fn!(AMBER, amber);

color_fn!(BORDER_SUBTLE, border_subtle);
color_fn!(BORDER_STRONG, border_strong);

color_fn!(SUCCESS, success);
color_fn!(ERROR, error);
color_fn!(WARNING, warning);

color_fn!(DIFF_ADD_BG, diff_add_bg);
color_fn!(DIFF_ADD_FG, diff_add_fg);
color_fn!(DIFF_DEL_BG, diff_del_bg);
color_fn!(DIFF_DEL_FG, diff_del_fg);
color_fn!(DIFF_HUNK_HEADER, diff_hunk_header);

color_fn!(SHELL_RUNNING_BG, shell_running_bg);
color_fn!(SHELL_SUCCESS_BG, shell_success_bg);
color_fn!(SHELL_ERROR_BG, shell_error_bg);
color_fn!(CHAT_BG, chat_bg);
color_fn!(SYSTEM_BG, system_bg);

color_fn!(BUTTON_BG, button_bg);
color_fn!(BUTTON_HOVER, button_hover);
color_fn!(INPUT_BG, input_bg);
color_fn!(TAB_ACTIVE_BG, tab_active_bg);
color_fn!(TAB_INACTIVE_BG, tab_inactive_bg);

// ── Public API ───────────────────────────────────────────────────

pub fn apply(ctx: &egui::Context, mode: ThemeMode) {
    let p = match mode {
        ThemeMode::Dark => Palette::DARK,
        ThemeMode::Light => Palette::LIGHT,
    };

    ACTIVE_PALETTE.with(|cell| cell.set(p));
    ACTIVE_MODE.with(|cell| cell.set(mode));

    let mut style = Style::default();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.window_margin = Margin::same(0);
    style.spacing.button_padding = Vec2::new(10.0, 5.0);
    style.visuals = build_visuals(&p, mode);
    ctx.set_style(style);
}

pub fn blend(base: Color32, tint: Color32, amount: f32) -> Color32 {
    let a = amount.clamp(0.0, 1.0);
    let keep = 1.0 - a;
    Color32::from_rgb(
        blend_ch(base.r(), tint.r(), keep, a),
        blend_ch(base.g(), tint.g(), keep, a),
        blend_ch(base.b(), tint.b(), keep, a),
    )
}

pub fn alpha(color: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}

pub fn panel_border(accent: Color32, focused: bool) -> Color32 {
    if focused {
        blend(BORDER_STRONG(), accent, 0.7)
    } else {
        alpha(blend(BORDER_SUBTLE(), accent, 0.25), 180)
    }
}

// ── Internal ─────────────────────────────────────────────────────

fn build_visuals(p: &Palette, mode: ThemeMode) -> Visuals {
    let is_light = mode == ThemeMode::Light;
    let mut v = if is_light {
        Visuals::light()
    } else {
        Visuals::dark()
    };

    v.window_corner_radius = CornerRadius::same(14);
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 2,
        color: Color32::from_black_alpha(if is_light { 40 } else { 120 }),
    };
    v.window_stroke = Stroke::new(1.0, p.border_subtle);
    v.window_fill = p.panel_bg;
    v.window_highlight_topmost = false;

    v.panel_fill = p.bg;
    v.faint_bg_color = p.panel_bg_alt;
    v.extreme_bg_color = p.bg;
    v.code_bg_color = p.bg_elevated;
    v.override_text_color = Some(p.fg);
    v.resize_corner_size = 14.0;

    v.widgets.noninteractive.bg_fill = p.bg_elevated;
    v.widgets.noninteractive.weak_bg_fill = p.panel_bg_alt;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.fg_dim);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(10);

    v.widgets.inactive.bg_fill = p.panel_bg_alt;
    v.widgets.inactive.weak_bg_fill = p.bg_elevated;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.fg_soft);
    v.widgets.inactive.corner_radius = CornerRadius::same(10);

    v.widgets.hovered.bg_fill = blend(p.panel_bg_alt, p.accent, 0.14);
    v.widgets.hovered.weak_bg_fill = p.bg_elevated;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, p.fg);
    v.widgets.hovered.corner_radius = CornerRadius::same(10);

    v.widgets.active.bg_fill = blend(p.panel_bg_alt, p.accent, 0.22);
    v.widgets.active.weak_bg_fill = p.bg_elevated;
    v.widgets.active.fg_stroke = Stroke::new(1.0, p.fg);
    v.widgets.active.corner_radius = CornerRadius::same(10);

    v.selection.bg_fill = alpha(p.accent, 50);
    v.selection.stroke = Stroke::new(1.0, p.accent);

    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(if is_light { 30 } else { 100 }),
    };

    v
}

fn blend_ch(base: u8, tint: u8, keep: f32, amount: f32) -> u8 {
    let mixed = (f32::from(base) * keep + f32::from(tint) * amount)
        .round()
        .clamp(0.0, 255.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        mixed as u8
    }
}
