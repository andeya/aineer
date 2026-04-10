use gpui::*;

use crate::workspace::{
    AineerWorkspace, ClearBlocks, CloseTab, FocusInput, NewTab, SwitchToAIMode, SwitchToAgentMode,
    SwitchToShellMode, ToggleSidebar,
};

pub fn run_app() {
    let app = Application::new();

    app.run(|cx| {
        crate::platform::set_dock_icon();

        let title = format!(
            "{} v{}",
            aineer_release_channel::ReleaseChannel::current().display_name(),
            env!("CARGO_PKG_VERSION"),
        );

        cx.bind_keys([
            KeyBinding::new("cmd-b", ToggleSidebar, Some("Workspace")),
            KeyBinding::new("cmd-t", NewTab, Some("Workspace")),
            KeyBinding::new("cmd-w", CloseTab, Some("Workspace")),
            KeyBinding::new("cmd-1", SwitchToShellMode, Some("Workspace")),
            KeyBinding::new("cmd-2", SwitchToAIMode, Some("Workspace")),
            KeyBinding::new("cmd-3", SwitchToAgentMode, Some("Workspace")),
            KeyBinding::new("cmd-l", FocusInput, Some("Workspace")),
            KeyBinding::new("cmd-k", ClearBlocks, Some("Workspace")),
        ]);

        let window_options = WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some(title.into()),
                ..Default::default()
            }),
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1200.0), px(800.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(window_options, |_window, cx| {
            cx.new(|cx| AineerWorkspace::new(cx))
        })
        .expect("Failed to open window");

        cx.activate(true);
    });
}
