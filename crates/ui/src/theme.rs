use egui::{Color32, CornerRadius, Margin, Shadow, Stroke, Style, Vec2, Visuals};

// ── Layout constants ─────────────────────────────────────────────
pub const TOOLBAR_HEIGHT: f32 = 38.0;
pub const PANEL_PADDING: f32 = 8.0;
pub const CARD_CORNER_RADIUS: f32 = 10.0;
pub const CARD_INNER_MARGIN: f32 = 12.0;
pub const BUTTON_CORNER_RADIUS: f32 = 8.0;
pub const INPUT_CORNER_RADIUS: f32 = 8.0;

// ── Base palette ─────────────────────────────────────────────────
pub const BG: Color32 = Color32::from_rgb(14, 14, 26);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(20, 21, 33);
pub const PANEL_BG: Color32 = Color32::from_rgb(24, 24, 36);
pub const PANEL_BG_ALT: Color32 = Color32::from_rgb(30, 30, 44);
pub const SURFACE: Color32 = Color32::from_rgb(36, 36, 52);

// ── Foreground ───────────────────────────────────────────────────
pub const FG: Color32 = Color32::from_rgb(224, 228, 240);
pub const FG_SOFT: Color32 = Color32::from_rgb(170, 178, 200);
pub const FG_DIM: Color32 = Color32::from_rgb(108, 116, 140);
pub const FG_MUTED: Color32 = Color32::from_rgb(80, 86, 110);

// ── Brand accent ─────────────────────────────────────────────────
pub const ACCENT: Color32 = Color32::from_rgb(99, 102, 241);
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(129, 140, 248);
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(6, 182, 212);
pub const AMBER: Color32 = Color32::from_rgb(251, 191, 36);

// ── Borders ──────────────────────────────────────────────────────
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(40, 42, 60);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(60, 64, 90);

// ── Semantic ─────────────────────────────────────────────────────
pub const SUCCESS: Color32 = Color32::from_rgb(78, 203, 113);
pub const ERROR: Color32 = Color32::from_rgb(243, 139, 168);
pub const WARNING: Color32 = Color32::from_rgb(233, 190, 109);

// ── Diff ─────────────────────────────────────────────────────────
pub const DIFF_ADD_BG: Color32 = Color32::from_rgb(24, 50, 32);
pub const DIFF_ADD_FG: Color32 = Color32::from_rgb(120, 220, 120);
pub const DIFF_DEL_BG: Color32 = Color32::from_rgb(55, 24, 28);
pub const DIFF_DEL_FG: Color32 = Color32::from_rgb(240, 130, 140);
pub const DIFF_HUNK_HEADER: Color32 = Color32::from_rgb(100, 150, 220);

// ── Shell card states ────────────────────────────────────────────
pub const SHELL_RUNNING_BG: Color32 = Color32::from_rgb(28, 28, 42);
pub const SHELL_SUCCESS_BG: Color32 = Color32::from_rgb(22, 36, 28);
pub const SHELL_ERROR_BG: Color32 = Color32::from_rgb(38, 22, 26);
pub const CHAT_BG: Color32 = Color32::from_rgb(24, 24, 42);
pub const SYSTEM_BG: Color32 = Color32::from_rgb(30, 28, 42);

// ── Interactive ──────────────────────────────────────────────────
pub const BUTTON_BG: Color32 = Color32::from_rgb(36, 38, 54);
pub const BUTTON_HOVER: Color32 = Color32::from_rgb(46, 48, 68);
pub const INPUT_BG: Color32 = Color32::from_rgb(20, 21, 33);
pub const TAB_ACTIVE_BG: Color32 = Color32::from_rgb(42, 42, 58);
pub const TAB_INACTIVE_BG: Color32 = Color32::from_rgb(24, 24, 36);

pub fn apply(ctx: &egui::Context) {
    let mut style = Style::default();

    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.window_margin = Margin::same(0);
    style.spacing.button_padding = Vec2::new(10.0, 5.0);
    style.visuals = visuals();

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
        blend(BORDER_STRONG, accent, 0.7)
    } else {
        alpha(blend(BORDER_SUBTLE, accent, 0.25), 180)
    }
}

fn visuals() -> Visuals {
    let mut v = Visuals::dark();

    v.window_corner_radius = CornerRadius::same(14);
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 2,
        color: Color32::from_black_alpha(120),
    };
    v.window_stroke = Stroke::new(1.0, BORDER_SUBTLE);
    v.window_fill = PANEL_BG;
    v.window_highlight_topmost = false;

    v.panel_fill = BG;
    v.faint_bg_color = PANEL_BG_ALT;
    v.extreme_bg_color = BG;
    v.code_bg_color = BG_ELEVATED;
    v.override_text_color = Some(FG);
    v.resize_corner_size = 14.0;

    v.widgets.noninteractive.bg_fill = BG_ELEVATED;
    v.widgets.noninteractive.weak_bg_fill = PANEL_BG_ALT;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, FG_DIM);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(10);

    v.widgets.inactive.bg_fill = PANEL_BG_ALT;
    v.widgets.inactive.weak_bg_fill = BG_ELEVATED;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, FG_SOFT);
    v.widgets.inactive.corner_radius = CornerRadius::same(10);

    v.widgets.hovered.bg_fill = blend(PANEL_BG_ALT, ACCENT, 0.14);
    v.widgets.hovered.weak_bg_fill = BG_ELEVATED;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, FG);
    v.widgets.hovered.corner_radius = CornerRadius::same(10);

    v.widgets.active.bg_fill = blend(PANEL_BG_ALT, ACCENT, 0.22);
    v.widgets.active.weak_bg_fill = BG_ELEVATED;
    v.widgets.active.fg_stroke = Stroke::new(1.0, FG);
    v.widgets.active.corner_radius = CornerRadius::same(10);

    v.selection.bg_fill = alpha(ACCENT, 50);
    v.selection.stroke = Stroke::new(1.0, ACCENT);

    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(100),
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
