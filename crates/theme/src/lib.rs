pub mod colors;
pub mod presets;

pub use colors::{ThemeAppearance, ThemeColors, ThemeStyles};

/// A complete theme definition
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub appearance: ThemeAppearance,
    pub colors: ThemeColors,
    pub styles: ThemeStyles,
}

impl Theme {
    pub fn dark_default() -> Self {
        Self {
            name: "Aineer Dark".into(),
            appearance: ThemeAppearance::Dark,
            colors: presets::aineer_dark_colors(),
            styles: ThemeStyles::default(),
        }
    }

    pub fn light_default() -> Self {
        Self {
            name: "Aineer Light".into(),
            appearance: ThemeAppearance::Light,
            colors: presets::aineer_light_colors(),
            styles: ThemeStyles::default(),
        }
    }
}
