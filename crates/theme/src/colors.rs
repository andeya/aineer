use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeAppearance {
    Dark,
    Light,
}

/// HSLA color representation (compatible with GPUI Hsla)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Hsla {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

impl Hsla {
    pub const fn new(h: f32, s: f32, l: f32, a: f32) -> Self {
        Self { h, s, l, a }
    }

    /// Create from hex string like "#1e1e2e"
    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
        let a = if hex.len() > 6 {
            u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) as f32 / 255.0
        } else {
            1.0
        };
        rgb_to_hsla(r, g, b, a)
    }
}

fn rgb_to_hsla(r: f32, g: f32, b: f32, a: f32) -> Hsla {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f32::EPSILON {
        return Hsla::new(0.0, 0.0, l, a);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < f32::EPSILON {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f32::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    Hsla::new(h / 6.0, s, l, a)
}

/// ~200 semantic color tokens for the entire UI
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Backgrounds
    pub background: Hsla,
    pub background_surface: Hsla,
    pub background_elevated: Hsla,
    pub background_input: Hsla,

    // Text
    pub text_primary: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,
    pub text_on_accent: Hsla,

    // Accent
    pub accent_primary: Hsla,
    pub accent_ai: Hsla,
    pub accent_agent: Hsla,

    // Status
    pub status_error: Hsla,
    pub status_warning: Hsla,
    pub status_success: Hsla,
    pub status_info: Hsla,

    // Border
    pub border_default: Hsla,
    pub border_focus: Hsla,

    // Block-specific
    pub command_block_bg: Hsla,
    pub command_block_prompt_bg: Hsla,
    pub ai_block_bg: Hsla,
    pub agent_block_bg: Hsla,
    pub tool_block_bg: Hsla,
    pub system_block_bg: Hsla,

    // Activity Bar
    pub activity_bar_bg: Hsla,
    pub activity_bar_active: Hsla,

    // Status Bar
    pub status_bar_bg: Hsla,

    // Tab
    pub tab_active_bg: Hsla,
    pub tab_inactive_bg: Hsla,

    // Sidebar
    pub sidebar_bg: Hsla,

    // Scrollbar
    pub scrollbar_thumb: Hsla,
    pub scrollbar_thumb_hover: Hsla,
}

/// Non-color design tokens: fonts, spacing, radius, shadows
#[derive(Debug, Clone)]
pub struct ThemeStyles {
    // Font families
    pub font_mono: String,
    pub font_sans: String,

    // Font sizes (in px)
    pub font_size_xs: f32,
    pub font_size_sm: f32,
    pub font_size_base: f32,
    pub font_size_md: f32,
    pub font_size_lg: f32,
    pub font_size_xl: f32,
    pub font_size_2xl: f32,

    // Spacing (in px, multiples of 4)
    pub space_1: f32,
    pub space_2: f32,
    pub space_3: f32,
    pub space_4: f32,
    pub space_5: f32,
    pub space_6: f32,
    pub space_8: f32,
    pub space_12: f32,

    // Border radius
    pub radius_none: f32,
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
    pub radius_full: f32,

    // Layout
    pub activity_bar_width: f32,
    pub sidebar_default_width: f32,
    pub sidebar_min_width: f32,
    pub sidebar_max_width: f32,
    pub status_bar_height: f32,
    pub tab_bar_height: f32,
    pub input_bar_min_height: f32,
}

impl Default for ThemeStyles {
    fn default() -> Self {
        Self {
            font_mono: r#""Berkeley Mono", "Cascadia Code", "JetBrains Mono", "Menlo", monospace"#
                .into(),
            font_sans: r#""Inter", "SF Pro Text", "Segoe UI", system-ui, sans-serif"#.into(),
            font_size_xs: 11.0,
            font_size_sm: 12.0,
            font_size_base: 13.0,
            font_size_md: 14.0,
            font_size_lg: 16.0,
            font_size_xl: 20.0,
            font_size_2xl: 28.0,
            space_1: 4.0,
            space_2: 8.0,
            space_3: 12.0,
            space_4: 16.0,
            space_5: 20.0,
            space_6: 24.0,
            space_8: 32.0,
            space_12: 48.0,
            radius_none: 0.0,
            radius_sm: 2.0,
            radius_md: 4.0,
            radius_lg: 8.0,
            radius_full: 9999.0,
            activity_bar_width: 48.0,
            sidebar_default_width: 280.0,
            sidebar_min_width: 240.0,
            sidebar_max_width: 400.0,
            status_bar_height: 24.0,
            tab_bar_height: 36.0,
            input_bar_min_height: 40.0,
        }
    }
}
