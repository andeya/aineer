use gpui::prelude::FluentBuilder;
use gpui::*;
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════
//  Actions
// ═══════════════════════════════════════════════════════════════════

actions!(
    workspace,
    [
        ToggleSidebar,
        NewTab,
        CloseTab,
        SwitchToShellMode,
        SwitchToAIMode,
        SwitchToAgentMode,
        FocusInput,
        ClearBlocks,
    ]
);

// ═══════════════════════════════════════════════════════════════════
//  Types
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidebarPanel {
    Explorer,
    Search,
    Git,
    Context,
    Memory,
}

impl SidebarPanel {
    fn label(self) -> &'static str {
        match self {
            Self::Explorer => "EXPLORER",
            Self::Search => "SEARCH",
            Self::Git => "GIT",
            Self::Context => "CONTEXT",
            Self::Memory => "MEMORY",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Self::Explorer => "E",
            Self::Search => "S",
            Self::Git => "G",
            Self::Context => "C",
            Self::Memory => "M",
        }
    }
}

const ALL_PANELS: [SidebarPanel; 5] = [
    SidebarPanel::Explorer,
    SidebarPanel::Search,
    SidebarPanel::Git,
    SidebarPanel::Context,
    SidebarPanel::Memory,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Shell,
    AIChat,
    Agent,
}

impl InputMode {
    fn label(self) -> &'static str {
        match self {
            Self::Shell => "Shell",
            Self::AIChat => "AI",
            Self::Agent => "Agent",
        }
    }
    fn next(self) -> Self {
        match self {
            Self::Shell => Self::AIChat,
            Self::AIChat => Self::Agent,
            Self::Agent => Self::Shell,
        }
    }
}

// ─── Stream Blocks ───

#[derive(Debug, Clone)]
enum StreamBlock {
    Command {
        text: String,
        cwd: String,
    },
    Output {
        text: String,
        exit_code: Option<i32>,
    },
    AiMessage {
        content: String,
        model: String,
    },
    System {
        message: String,
    },
    Error {
        message: String,
    },
}

// ─── File Tree ───

#[derive(Debug, Clone)]
struct FileNode {
    name: String,
    path: PathBuf,
    is_dir: bool,
    depth: usize,
    expanded: bool,
    children_loaded: bool,
}

struct TabInfo {
    id: usize,
    title: String,
}

// ═══════════════════════════════════════════════════════════════════
//  Color Palette (§2.1 Design Tokens)
// ═══════════════════════════════════════════════════════════════════

struct Clr;
impl Clr {
    const BG: u32 = 0x1e1e2e;
    const SURFACE: u32 = 0x232334;
    const ELEVATED: u32 = 0x2a2a3c;
    const TEXT: u32 = 0xe0e0e8;
    const TEXT2: u32 = 0x8888a0;
    const MUTED: u32 = 0x5c5c72;
    const ACCENT: u32 = 0x5b9cf5;
    const AI: u32 = 0xb07aff;
    const AGENT: u32 = 0xf0a050;
    const BORDER: u32 = 0x3a3a4e;
    const SUCCESS: u32 = 0x50c878;
    const ERROR: u32 = 0xf44747;
    const BAR: u32 = 0x1a1a28;
    const HOVER: u32 = 0x2e2e40;
}

// ═══════════════════════════════════════════════════════════════════
//  Workspace
// ═══════════════════════════════════════════════════════════════════

pub struct AineerWorkspace {
    focus_handle: FocusHandle,
    scroll_handle: ScrollHandle,

    // Tabs
    tabs: Vec<TabInfo>,
    active_tab: usize,
    next_tab_id: usize,

    // Sidebar
    sidebar_visible: bool,
    sidebar_width: f32,
    active_panel: SidebarPanel,

    // Input
    input_mode: InputMode,
    input_text: String,
    cursor_pos: usize,
    command_history: Vec<String>,
    history_index: Option<usize>,
    saved_input: String,

    // Blocks (unified stream)
    blocks: Vec<StreamBlock>,

    // File explorer
    file_tree: Vec<FileNode>,
    workspace_root: PathBuf,

    // Flags
    needs_initial_focus: bool,
}

impl AineerWorkspace {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let file_tree = load_directory(&workspace_root, 0);

        Self {
            focus_handle,
            scroll_handle: ScrollHandle::new(),
            tabs: vec![TabInfo {
                id: 0,
                title: "Terminal".into(),
            }],
            active_tab: 0,
            next_tab_id: 1,
            sidebar_visible: true,
            sidebar_width: 260.0,
            active_panel: SidebarPanel::Explorer,
            input_mode: InputMode::Shell,
            input_text: String::new(),
            cursor_pos: 0,
            command_history: Vec::new(),
            history_index: None,
            saved_input: String::new(),
            blocks: Vec::new(),
            file_tree,
            workspace_root,
            needs_initial_focus: true,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Action Handlers
// ═══════════════════════════════════════════════════════════════════

impl AineerWorkspace {
    fn on_toggle_sidebar(&mut self, _: &ToggleSidebar, _w: &mut Window, cx: &mut Context<Self>) {
        self.sidebar_visible = !self.sidebar_visible;
        cx.notify();
    }
    fn on_new_tab(&mut self, _: &NewTab, _w: &mut Window, cx: &mut Context<Self>) {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(TabInfo {
            id,
            title: format!("Terminal {}", id),
        });
        self.active_tab = self.tabs.len() - 1;
        cx.notify();
    }
    fn on_close_tab(&mut self, _: &CloseTab, _w: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.len() > 1 {
            self.tabs.remove(self.active_tab);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
            cx.notify();
        }
    }
    fn on_shell_mode(&mut self, _: &SwitchToShellMode, _w: &mut Window, cx: &mut Context<Self>) {
        self.input_mode = InputMode::Shell;
        cx.notify();
    }
    fn on_ai_mode(&mut self, _: &SwitchToAIMode, _w: &mut Window, cx: &mut Context<Self>) {
        self.input_mode = InputMode::AIChat;
        cx.notify();
    }
    fn on_agent_mode(&mut self, _: &SwitchToAgentMode, _w: &mut Window, cx: &mut Context<Self>) {
        self.input_mode = InputMode::Agent;
        cx.notify();
    }
    fn on_focus_input(&mut self, _: &FocusInput, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window);
        cx.notify();
    }
    fn on_clear_blocks(&mut self, _: &ClearBlocks, _w: &mut Window, cx: &mut Context<Self>) {
        self.blocks.clear();
        cx.notify();
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Keyboard Input
// ═══════════════════════════════════════════════════════════════════

impl AineerWorkspace {
    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        let mods = &event.keystroke.modifiers;

        match key {
            "enter" if !mods.shift && !mods.platform => {
                self.submit_input(cx);
            }
            "backspace" => {
                if self.cursor_pos > 0 {
                    let remove_pos = self.input_text[..self.cursor_pos]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.input_text.remove(remove_pos);
                    self.cursor_pos = remove_pos;
                }
            }
            "left" => {
                if self.cursor_pos > 0 {
                    self.cursor_pos = self.input_text[..self.cursor_pos]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                }
            }
            "right" => {
                if self.cursor_pos < self.input_text.len() {
                    self.cursor_pos += self.input_text[self.cursor_pos..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                }
            }
            "up" => {
                self.history_prev();
            }
            "down" => {
                self.history_next();
            }
            "escape" => {
                if !self.input_text.is_empty() {
                    self.input_text.clear();
                    self.cursor_pos = 0;
                } else {
                    window.blur();
                }
            }
            "a" if mods.control => {
                self.cursor_pos = 0;
            }
            "e" if mods.control => {
                self.cursor_pos = self.input_text.len();
            }
            "u" if mods.control => {
                self.input_text.drain(..self.cursor_pos);
                self.cursor_pos = 0;
            }
            "k" if mods.control => {
                self.input_text.truncate(self.cursor_pos);
            }
            "l" if mods.control => {
                self.blocks.clear();
            }
            _ => {
                if mods.platform || mods.function {
                    return; // let actions handle cmd+key
                }
                if let Some(ch) = &event.keystroke.key_char {
                    if !ch.is_empty() {
                        self.input_text.insert_str(self.cursor_pos, ch);
                        self.cursor_pos += ch.len();
                        self.history_index = None;
                    }
                }
            }
        }

        cx.stop_propagation();
        cx.notify();
    }

    fn history_prev(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        match self.history_index {
            None => {
                self.saved_input = self.input_text.clone();
                self.history_index = Some(self.command_history.len() - 1);
            }
            Some(0) => return,
            Some(ref mut idx) => *idx -= 1,
        }
        if let Some(idx) = self.history_index {
            self.input_text = self.command_history[idx].clone();
            self.cursor_pos = self.input_text.len();
        }
    }

    fn history_next(&mut self) {
        match self.history_index {
            None => return,
            Some(idx) => {
                if idx + 1 >= self.command_history.len() {
                    self.history_index = None;
                    self.input_text = self.saved_input.clone();
                } else {
                    self.history_index = Some(idx + 1);
                    self.input_text = self.command_history[idx + 1].clone();
                }
            }
        }
        self.cursor_pos = self.input_text.len();
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Command Execution
// ═══════════════════════════════════════════════════════════════════

impl AineerWorkspace {
    fn submit_input(&mut self, cx: &mut Context<Self>) {
        let text = self.input_text.trim().to_string();
        if text.is_empty() {
            return;
        }

        self.input_text.clear();
        self.cursor_pos = 0;
        self.history_index = None;

        match self.input_mode {
            InputMode::Shell => {
                self.command_history.push(text.clone());
                self.execute_shell(&text, cx);
            }
            InputMode::AIChat => {
                self.blocks.push(StreamBlock::Command {
                    text: text.clone(),
                    cwd: String::new(),
                });
                self.blocks.push(StreamBlock::AiMessage {
                    content: format!(
                        "AI chat is not yet connected. You said: \"{}\".\n\
                         Configure a provider in Settings to enable.",
                        text
                    ),
                    model: "none".into(),
                });
                self.scroll_handle.scroll_to_bottom();
                cx.notify();
            }
            InputMode::Agent => {
                self.blocks.push(StreamBlock::Command {
                    text: text.clone(),
                    cwd: String::new(),
                });
                self.blocks.push(StreamBlock::System {
                    message: format!("Agent mode coming soon. Task received: \"{}\"", text),
                });
                self.scroll_handle.scroll_to_bottom();
                cx.notify();
            }
        }
    }

    fn execute_shell(&mut self, command: &str, cx: &mut Context<Self>) {
        let cwd = std::env::current_dir().unwrap_or_else(|_| self.workspace_root.clone());

        // Handle built-in commands
        if command == "clear" {
            self.blocks.clear();
            cx.notify();
            return;
        }
        if let Some(dir) =
            command
                .strip_prefix("cd ")
                .or_else(|| if command == "cd" { Some("") } else { None })
        {
            self.handle_cd(dir.trim(), &cwd, cx);
            return;
        }

        self.blocks.push(StreamBlock::Command {
            text: command.to_string(),
            cwd: short_path(&cwd),
        });

        let output_idx = self.blocks.len();
        self.blocks.push(StreamBlock::Output {
            text: String::new(),
            exit_code: None,
        });
        self.scroll_handle.scroll_to_bottom();
        cx.notify();

        let cmd = command.to_string();
        cx.spawn(async move |entity, cx| {
            let result = smol::unblock(move || {
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .current_dir(&cwd)
                    .output()
            })
            .await;

            let _ = cx.update(|cx| {
                if let Some(entity) = entity.upgrade() {
                    entity.update(cx, |this, cx| {
                        match result {
                            Ok(out) => {
                                let mut text = String::from_utf8_lossy(&out.stdout).to_string();
                                let stderr = String::from_utf8_lossy(&out.stderr);
                                if !stderr.is_empty() {
                                    if !text.is_empty() {
                                        text.push('\n');
                                    }
                                    text.push_str(&stderr);
                                }
                                if let Some(block) = this.blocks.get_mut(output_idx) {
                                    *block = StreamBlock::Output {
                                        text: text.trim_end().to_string(),
                                        exit_code: out.status.code(),
                                    };
                                }
                            }
                            Err(e) => {
                                if let Some(block) = this.blocks.get_mut(output_idx) {
                                    *block = StreamBlock::Error {
                                        message: format!("exec error: {}", e),
                                    };
                                }
                            }
                        }
                        this.scroll_handle.scroll_to_bottom();
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn handle_cd(&mut self, dir: &str, cwd: &PathBuf, cx: &mut Context<Self>) {
        let target = if dir.is_empty() || dir == "~" {
            dirs_or_home()
        } else if dir.starts_with('/') {
            PathBuf::from(dir)
        } else if dir.starts_with("~/") {
            dirs_or_home().join(&dir[2..])
        } else {
            cwd.join(dir)
        };

        self.blocks.push(StreamBlock::Command {
            text: format!("cd {}", dir),
            cwd: short_path(cwd),
        });

        match std::env::set_current_dir(&target) {
            Ok(()) => {
                self.workspace_root = target.clone();
                self.file_tree = load_directory(&target, 0);
                self.blocks.push(StreamBlock::System {
                    message: format!("→ {}", target.display()),
                });
            }
            Err(e) => {
                self.blocks.push(StreamBlock::Error {
                    message: format!("cd: {}: {}", dir, e),
                });
            }
        }
        self.scroll_handle.scroll_to_bottom();
        cx.notify();
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Render — Root
// ═══════════════════════════════════════════════════════════════════

impl Render for AineerWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.needs_initial_focus {
            self.focus_handle.focus(window);
            self.needs_initial_focus = false;
        }

        div()
            .id("workspace-root")
            .key_context("Workspace")
            .size_full()
            .bg(rgb(Clr::BG))
            .text_color(rgb(Clr::TEXT))
            .font_family("Berkeley Mono, JetBrains Mono, Menlo, monospace")
            .flex()
            .flex_row()
            .on_action(cx.listener(Self::on_toggle_sidebar))
            .on_action(cx.listener(Self::on_new_tab))
            .on_action(cx.listener(Self::on_close_tab))
            .on_action(cx.listener(Self::on_shell_mode))
            .on_action(cx.listener(Self::on_ai_mode))
            .on_action(cx.listener(Self::on_agent_mode))
            .on_action(cx.listener(Self::on_focus_input))
            .on_action(cx.listener(Self::on_clear_blocks))
            .child(self.render_activity_bar(cx))
            .child(self.render_main(window, cx))
            .when(self.sidebar_visible, |el| el.child(self.render_sidebar(cx)))
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Render — Activity Bar
// ═══════════════════════════════════════════════════════════════════

impl AineerWorkspace {
    fn render_activity_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("bar-activity")
            .w(px(44.0))
            .h_full()
            .bg(rgb(Clr::BAR))
            .border_r_1()
            .border_color(rgb(Clr::BORDER))
            .flex()
            .flex_col()
            .items_center()
            .pt(px(8.0))
            .gap(px(2.0))
            .children(ALL_PANELS.iter().map(|&panel| {
                let active = panel == self.active_panel && self.sidebar_visible;
                div()
                    .id(SharedString::from(format!("p-{}", panel.icon())))
                    .w(px(36.0))
                    .h(px(32.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(14.0))
                    .text_color(if active {
                        rgb(Clr::TEXT)
                    } else {
                        rgb(Clr::MUTED)
                    })
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(Clr::HOVER)).text_color(rgb(Clr::TEXT)))
                    .when(active, |el| {
                        el.bg(rgb(Clr::HOVER))
                            .border_l_2()
                            .border_color(rgb(Clr::ACCENT))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.active_panel == panel && this.sidebar_visible {
                            this.sidebar_visible = false;
                        } else {
                            this.active_panel = panel;
                            this.sidebar_visible = true;
                        }
                        cx.notify();
                    }))
                    .child(panel.icon())
            }))
            .child(div().flex_1()) // spacer
            .child(
                div()
                    .id("btn-settings")
                    .w(px(36.0))
                    .h(px(32.0))
                    .mb(px(8.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(14.0))
                    .text_color(rgb(Clr::MUTED))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(Clr::HOVER)).text_color(rgb(Clr::TEXT)))
                    .child("⚙"),
            )
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Render — Main Area
// ═══════════════════════════════════════════════════════════════════

impl AineerWorkspace {
    fn render_main(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("main-area")
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .child(self.render_tab_bar(cx))
            .child(self.render_content(cx))
            .child(self.render_input_bar(window, cx))
            .child(self.render_status_bar())
    }

    // ─── Tab Bar ───
    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("tab-bar")
            .w_full()
            .h(px(34.0))
            .bg(rgb(Clr::BG))
            .border_b_1()
            .border_color(rgb(Clr::BORDER))
            .flex()
            .flex_row()
            .items_center()
            .px(px(4.0))
            .children(self.tabs.iter().enumerate().map(|(i, tab)| {
                let active = i == self.active_tab;
                let tab_i = i;
                let tab_id = tab.id;
                div()
                    .id(SharedString::from(format!("tab-{}", tab_id)))
                    .px(px(12.0))
                    .py(px(6.0))
                    .text_size(px(12.0))
                    .text_color(if active {
                        rgb(Clr::TEXT)
                    } else {
                        rgb(Clr::TEXT2)
                    })
                    .rounded_t(px(3.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(Clr::ELEVATED)))
                    .when(active, |el| {
                        el.bg(rgb(Clr::SURFACE))
                            .border_b_2()
                            .border_color(rgb(Clr::ACCENT))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.active_tab = tab_i;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .child(tab.title.clone())
                            .when(active && self.tabs.len() > 1, |el| {
                                el.child(
                                    div()
                                        .id(SharedString::from(format!("x-{}", tab_id)))
                                        .text_size(px(10.0))
                                        .text_color(rgb(Clr::MUTED))
                                        .w(px(14.0))
                                        .h(px(14.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(2.0))
                                        .cursor_pointer()
                                        .hover(|s| {
                                            s.bg(rgb(Clr::BORDER)).text_color(rgb(Clr::TEXT))
                                        })
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if this.tabs.len() > 1 {
                                                this.tabs.remove(tab_i);
                                                if this.active_tab >= this.tabs.len() {
                                                    this.active_tab = this.tabs.len() - 1;
                                                }
                                                cx.notify();
                                            }
                                        }))
                                        .child("×"),
                                )
                            }),
                    )
            }))
            .child(
                div()
                    .id("btn-new-tab")
                    .px(px(8.0))
                    .py(px(6.0))
                    .text_size(px(14.0))
                    .text_color(rgb(Clr::MUTED))
                    .rounded(px(3.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(Clr::ELEVATED)).text_color(rgb(Clr::TEXT)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        let id = this.next_tab_id;
                        this.next_tab_id += 1;
                        this.tabs.push(TabInfo {
                            id,
                            title: format!("Terminal {}", id),
                        });
                        this.active_tab = this.tabs.len() - 1;
                        cx.notify();
                    }))
                    .child("+"),
            )
    }

    // ─── Content Area (blocks or welcome) ───
    fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if self.blocks.is_empty() {
            self.render_welcome(cx).into_any_element()
        } else {
            self.render_block_stream().into_any_element()
        }
    }

    fn render_welcome(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("content-welcome")
            .flex_1()
            .w_full()
            .bg(rgb(Clr::BG))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(16.0))
            .child(
                div()
                    .px(px(24.0))
                    .py(px(12.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(rgb(Clr::BORDER))
                    .bg(rgb(Clr::SURFACE))
                    .child(
                        div()
                            .text_size(px(32.0))
                            .text_color(rgb(Clr::ACCENT))
                            .child("◆ AINEER"),
                    ),
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(rgb(Clr::TEXT2))
                    .child("The Agentic Development Environment"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(Clr::MUTED))
                    .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
            )
            .child(
                div()
                    .mt(px(20.0))
                    .flex()
                    .flex_row()
                    .gap(px(10.0))
                    .child(self.render_card("⌨", "Shell", "Type a command", cx))
                    .child(self.render_card("✦", "AI Chat", "Ask anything", cx))
                    .child(self.render_card("⚡", "Agent", "Automate a task", cx)),
            )
            .child(
                div()
                    .mt(px(16.0))
                    .flex()
                    .flex_row()
                    .gap(px(14.0))
                    .text_size(px(11.0))
                    .text_color(rgb(Clr::MUTED))
                    .child("↑↓ History")
                    .child("⌘B Sidebar")
                    .child("⌘T New Tab")
                    .child("Ctrl+L Clear")
                    .child("Ctrl+A/E Home/End"),
            )
    }

    fn render_card(
        &self,
        icon: &str,
        title: &str,
        sub: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mode = match title {
            "AI Chat" => Some(InputMode::AIChat),
            "Agent" => Some(InputMode::Agent),
            _ => Some(InputMode::Shell),
        };

        div()
            .id(SharedString::from(format!("card-{}", title)))
            .w(px(150.0))
            .px(px(14.0))
            .py(px(12.0))
            .bg(rgb(Clr::SURFACE))
            .rounded(px(6.0))
            .border_1()
            .border_color(rgb(Clr::BORDER))
            .cursor_pointer()
            .hover(|s| s.bg(rgb(Clr::ELEVATED)).border_color(rgb(Clr::ACCENT)))
            .flex()
            .flex_col()
            .gap(px(3.0))
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(m) = mode {
                    this.input_mode = m;
                }
                cx.notify();
            }))
            .child(
                div()
                    .text_size(px(18.0))
                    .text_color(rgb(Clr::ACCENT))
                    .child(icon.to_string()),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(rgb(Clr::TEXT))
                    .child(title.to_string()),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(Clr::MUTED))
                    .child(sub.to_string()),
            )
    }

    fn render_block_stream(&self) -> impl IntoElement {
        div()
            .id("content-blocks")
            .flex_1()
            .w_full()
            .bg(rgb(Clr::BG))
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .flex()
            .flex_col()
            .pb(px(8.0))
            .children(
                self.blocks
                    .iter()
                    .enumerate()
                    .map(|(i, block)| render_block(i, block)),
            )
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Render — Blocks
// ═══════════════════════════════════════════════════════════════════

fn render_block(idx: usize, block: &StreamBlock) -> impl IntoElement {
    let id = SharedString::from(format!("blk-{}", idx));

    match block {
        StreamBlock::Command { text, cwd } => div()
            .id(id)
            .w_full()
            .px(px(16.0))
            .pt(px(10.0))
            .pb(px(2.0))
            .flex()
            .flex_row()
            .items_baseline()
            .gap(px(8.0))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(Clr::MUTED))
                    .when(!cwd.is_empty(), |el| el.child(cwd.clone()))
                    .when(cwd.is_empty(), |el| el.child("AI")),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(rgb(Clr::ACCENT))
                    .child(format!("❯ {}", text)),
            ),

        StreamBlock::Output { text, exit_code } => {
            let color = match exit_code {
                Some(0) | None => Clr::TEXT2,
                _ => Clr::ERROR,
            };
            let badge = match exit_code {
                Some(0) => None,
                Some(code) => Some(format!("exit {}", code)),
                None => None,
            };

            div()
                .id(id)
                .w_full()
                .px(px(16.0))
                .pt(px(2.0))
                .pb(px(6.0))
                .flex()
                .flex_col()
                .when(!text.is_empty(), |el| {
                    el.child(
                        div()
                            .w_full()
                            .px(px(12.0))
                            .py(px(6.0))
                            .bg(rgb(Clr::SURFACE))
                            .rounded(px(4.0))
                            .text_size(px(12.0))
                            .text_color(rgb(color))
                            .child(text.clone()),
                    )
                })
                .when(text.is_empty(), |el| {
                    el.child(
                        div()
                            .px(px(12.0))
                            .py(px(4.0))
                            .text_size(px(11.0))
                            .text_color(rgb(Clr::MUTED))
                            .child("⏳ Running..."),
                    )
                })
                .when(badge.is_some(), |el| {
                    el.child(
                        div()
                            .px(px(12.0))
                            .pt(px(2.0))
                            .text_size(px(10.0))
                            .text_color(rgb(Clr::ERROR))
                            .child(badge.unwrap_or_default()),
                    )
                })
        }

        StreamBlock::AiMessage { content, model } => div()
            .id(id)
            .w_full()
            .px(px(16.0))
            .py(px(6.0))
            .flex()
            .flex_row()
            .gap(px(8.0))
            .child(
                div()
                    .w(px(20.0))
                    .h(px(20.0))
                    .rounded(px(10.0))
                    .bg(rgb(Clr::AI))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(10.0))
                    .text_color(rgb(0xffffff))
                    .child("AI"),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(rgb(Clr::AI))
                            .child(model.clone()),
                    )
                    .child(
                        div()
                            .px(px(10.0))
                            .py(px(6.0))
                            .bg(rgb(Clr::SURFACE))
                            .rounded(px(4.0))
                            .text_size(px(13.0))
                            .text_color(rgb(Clr::TEXT))
                            .child(content.clone()),
                    ),
            ),

        StreamBlock::System { message } => div()
            .id(id)
            .w_full()
            .px(px(16.0))
            .py(px(4.0))
            .flex()
            .justify_center()
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(Clr::MUTED))
                    .child(format!("— {} —", message)),
            ),

        StreamBlock::Error { message } => div().id(id).w_full().px(px(16.0)).py(px(4.0)).child(
            div()
                .px(px(10.0))
                .py(px(4.0))
                .bg(rgb(0x3d1a1a))
                .rounded(px(4.0))
                .text_size(px(12.0))
                .text_color(rgb(Clr::ERROR))
                .child(format!("✗ {}", message)),
        ),
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Render — Input Bar
// ═══════════════════════════════════════════════════════════════════

impl AineerWorkspace {
    fn render_input_bar(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        let mode_color = match self.input_mode {
            InputMode::Shell => rgb(Clr::TEXT2),
            InputMode::AIChat => rgb(Clr::AI),
            InputMode::Agent => rgb(Clr::AGENT),
        };
        let border_clr = if focused {
            mode_color
        } else {
            rgb(Clr::BORDER)
        };

        // Split input text at cursor for rendering
        let (before_cursor, after_cursor) = self
            .input_text
            .split_at(self.cursor_pos.min(self.input_text.len()));
        let before = before_cursor.to_string();
        let after = after_cursor.to_string();
        let is_empty = self.input_text.is_empty();

        div()
            .id("input-bar")
            .key_context("InputBar")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_key_down))
            .w_full()
            .min_h(px(42.0))
            .bg(rgb(Clr::SURFACE))
            .border_t_1()
            .border_color(border_clr)
            .flex()
            .flex_row()
            .items_center()
            .px(px(10.0))
            .gap(px(8.0))
            .cursor_text()
            // Mode badge
            .child(
                div()
                    .id("input-mode")
                    .px(px(7.0))
                    .py(px(2.0))
                    .text_size(px(11.0))
                    .text_color(mode_color)
                    .bg(rgb(Clr::ELEVATED))
                    .rounded(px(3.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(Clr::BORDER)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.input_mode = this.input_mode.next();
                        cx.notify();
                    }))
                    .child(format!("[{}]", self.input_mode.label())),
            )
            // Text + cursor
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .items_center()
                    .overflow_x_hidden()
                    .when(is_empty, |el| {
                        let ph = match self.input_mode {
                            InputMode::Shell => "❯ Type a command...",
                            InputMode::AIChat => "Ask AI anything...",
                            InputMode::Agent => "Describe a task for Agent...",
                        };
                        el.child(
                            div()
                                .text_size(px(13.0))
                                .text_color(rgb(Clr::MUTED))
                                .child(ph),
                        )
                    })
                    .when(!is_empty || focused, |el| {
                        el.when(!is_empty, |el| {
                            el.child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(rgb(Clr::TEXT))
                                    .child(before),
                            )
                        })
                        .when(focused, |el| {
                            el.child(div().w(px(1.5)).h(px(16.0)).bg(mode_color))
                        })
                        .when(!after.is_empty(), |el| {
                            el.child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(rgb(Clr::TEXT))
                                    .child(after),
                            )
                        })
                    }),
            )
            // Right hints
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .text_size(px(10.0))
                    .text_color(rgb(Clr::MUTED))
                    .child(
                        div()
                            .id("hint-mode")
                            .px(px(5.0))
                            .py(px(1.0))
                            .rounded(px(2.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(Clr::ELEVATED)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.input_mode = this.input_mode.next();
                                cx.notify();
                            }))
                            .child(match self.input_mode {
                                InputMode::Shell => "⌘⏎ AI".to_string(),
                                InputMode::AIChat => "⌘⇧⏎ Agent".to_string(),
                                InputMode::Agent => "Esc Shell".to_string(),
                            }),
                    ),
            )
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Render — Status Bar
// ═══════════════════════════════════════════════════════════════════

impl AineerWorkspace {
    fn render_status_bar(&self) -> impl IntoElement {
        let cwd = std::env::current_dir().ok();
        let cwd_str = cwd
            .as_ref()
            .and_then(|p| p.file_name().map(|n| format!("~/{}", n.to_string_lossy())))
            .unwrap_or_else(|| "~/".into());

        div()
            .id("status-bar")
            .w_full()
            .h(px(24.0))
            .bg(rgb(Clr::BAR))
            .border_t_1()
            .border_color(rgb(Clr::BORDER))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(10.0))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(Clr::TEXT2))
                    .flex()
                    .flex_row()
                    .gap(px(10.0))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .w(px(6.0))
                                    .h(px(6.0))
                                    .rounded(px(3.0))
                                    .bg(rgb(Clr::SUCCESS)),
                            )
                            .child("Gateway"),
                    )
                    .child("main")
                    .child(cwd_str),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(Clr::TEXT2))
                    .flex()
                    .flex_row()
                    .gap(px(10.0))
                    .child(
                        div()
                            .text_color(match self.input_mode {
                                InputMode::Shell => rgb(Clr::TEXT2),
                                InputMode::AIChat => rgb(Clr::AI),
                                InputMode::Agent => rgb(Clr::AGENT),
                            })
                            .child(self.input_mode.label()),
                    )
                    .child(format!("{} blocks", self.blocks.len())),
            )
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Render — Sidebar
// ═══════════════════════════════════════════════════════════════════

impl AineerWorkspace {
    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("sidebar")
            .w(px(self.sidebar_width))
            .h_full()
            .bg(rgb(Clr::BG))
            .border_l_1()
            .border_color(rgb(Clr::BORDER))
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .px(px(10.0))
                    .py(px(6.0))
                    .text_size(px(11.0))
                    .text_color(rgb(Clr::TEXT2))
                    .border_b_1()
                    .border_color(rgb(Clr::BORDER))
                    .child(self.active_panel.label()),
            )
            .child(match self.active_panel {
                SidebarPanel::Explorer => self.render_explorer(cx).into_any_element(),
                _ => div()
                    .flex_1()
                    .p(px(12.0))
                    .text_size(px(12.0))
                    .text_color(rgb(Clr::MUTED))
                    .child(format!("{} — coming soon", self.active_panel.label()))
                    .into_any_element(),
            })
    }

    fn render_explorer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("explorer-tree")
            .flex_1()
            .overflow_y_scroll()
            .py(px(4.0))
            .children(
                self.file_tree
                    .iter()
                    .enumerate()
                    .map(|(i, node)| self.render_file_node(i, node, cx)),
            )
    }

    fn render_file_node(
        &self,
        idx: usize,
        node: &FileNode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let indent = px(10.0 + node.depth as f32 * 14.0);
        let icon = if node.is_dir {
            if node.expanded {
                "▾ 📂"
            } else {
                "▸ 📁"
            }
        } else {
            file_icon(&node.name)
        };

        let path = node.path.clone();
        let is_dir = node.is_dir;

        div()
            .id(SharedString::from(format!("fn-{}", idx)))
            .w_full()
            .h(px(24.0))
            .pl(indent)
            .pr(px(8.0))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(4.0))
            .text_size(px(12.0))
            .text_color(if node.is_dir {
                rgb(Clr::TEXT)
            } else {
                rgb(Clr::TEXT2)
            })
            .cursor_pointer()
            .hover(|s| s.bg(rgb(Clr::HOVER)))
            .on_click(cx.listener(move |this, _, _, cx| {
                if is_dir {
                    this.toggle_dir(&path, cx);
                }
                cx.notify();
            }))
            .child(div().text_size(px(11.0)).child(icon))
            .child(node.name.clone())
    }

    fn toggle_dir(&mut self, path: &PathBuf, _cx: &mut Context<Self>) {
        if let Some(node) = self
            .file_tree
            .iter_mut()
            .find(|n| n.path == *path && n.is_dir)
        {
            if node.expanded {
                node.expanded = false;
                let depth = node.depth;
                let path = node.path.clone();
                self.file_tree
                    .retain(|n| !(n.depth > depth && n.path.starts_with(&path) && n.path != path));
            } else {
                node.expanded = true;
                if !node.children_loaded {
                    node.children_loaded = true;
                }
                let children = load_directory(&path, node.depth + 1);
                let pos = self
                    .file_tree
                    .iter()
                    .position(|n| n.path == *path)
                    .unwrap_or(0);
                for (j, child) in children.into_iter().enumerate() {
                    self.file_tree.insert(pos + 1 + j, child);
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
//  File System Helpers
// ═══════════════════════════════════════════════════════════════════

fn load_directory(path: &PathBuf, depth: usize) -> Vec<FileNode> {
    let mut entries = Vec::new();
    let Ok(read_dir) = std::fs::read_dir(path) else {
        return entries;
    };
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue; // hide dotfiles by default
        }
        if matches!(name.as_str(), "target" | "node_modules" | "__pycache__") {
            continue;
        }
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let node = FileNode {
            name,
            path: entry.path(),
            is_dir,
            depth,
            expanded: false,
            children_loaded: false,
        };
        if is_dir {
            dirs.push(node);
        } else {
            files.push(node);
        }
    }
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    entries.extend(dirs);
    entries.extend(files);
    entries
}

fn file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "  🦀",
        "toml" => "  ⚙",
        "md" => "  📝",
        "json" => "  {}",
        "yaml" | "yml" => "  📋",
        "sh" | "bash" => "  🐚",
        "py" => "  🐍",
        "js" | "ts" | "tsx" | "jsx" => "  ⬡",
        "svg" | "png" | "jpg" | "ico" | "icns" => "  🖼",
        "lock" => "  🔒",
        "txt" => "  📄",
        "gitignore" => "  🙈",
        _ => "  📄",
    }
}

fn short_path(path: &PathBuf) -> String {
    let home = dirs_or_home();
    if let Ok(stripped) = path.strip_prefix(&home) {
        format!("~/{}", stripped.display())
    } else {
        path.display().to_string()
    }
}

fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
