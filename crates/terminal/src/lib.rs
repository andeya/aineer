mod backend;
mod bindings;
mod font;
mod render;
mod theme;
mod types;
mod view;

pub use backend::settings::BackendSettings;
pub use backend::{
    BackendCommand, PtyEvent, RenderableContent, StyledLine, StyledSegment, TerminalBackend,
    TerminalMode, TerminalSize,
};
pub use bindings::{Binding, BindingAction, InputKind, KeyboardBinding};
pub use font::{FontSettings, TerminalFont};
pub use render::TerminalGridCache;
pub use theme::{ColorPalette, TerminalTheme};
pub use view::TerminalView;
