use std::path::PathBuf;
use std::sync::{mpsc, Arc};

use eframe::egui;
use egui::{FontId, Key, RichText, Vec2};

use terminal::{BackendCommand, FontSettings, PtyEvent, TerminalFont, TerminalView};
use ui::cards::{Card, ChatCard, ShellCard, SystemCard};
use ui::diff_panel::DiffPanel;
use ui::git_watcher::GitWatcher;
use ui::input_bar::{CardPickerItem, InputBar, SlashMenuItem, SubmitAction};
use ui::search_bar::{SearchAction, SearchBar};
use ui::settings::SettingsPanel;
use ui::timeline::{Timeline, TimelineAction};

use crate::agent::{AgentEvent, AgentHandle, ToolApproval};
use crate::ssh::{SshManager, SshProfile};
use crate::tabs::TabManager;
use crate::theme;

struct ActiveStream {
    tab_id: u64,
    card_id: u64,
    event_rx: mpsc::Receiver<AgentEvent>,
    approval_tx: tokio::sync::mpsc::Sender<ToolApproval>,
}

pub struct TabState {
    pub timeline: Timeline,
    pub input_bar: InputBar,
    pub fullscreen_overlay: bool,
    pub search_bar: SearchBar,
}

impl TabState {
    fn new() -> Self {
        let mut timeline = Timeline::new();
        let welcome_id = timeline.next_card_id();
        timeline.push_card(Card::System(SystemCard::new(
            welcome_id,
            format!(
                "Welcome to {} — {}\nPress Enter to run shell commands, Ctrl+Enter to chat with AI.",
                crate::branding::APP_NAME,
                crate::branding::APP_TAGLINE
            ),
        )));

        Self {
            timeline,
            input_bar: InputBar::new(),
            fullscreen_overlay: false,
            search_bar: SearchBar::new(),
        }
    }

    fn from_tab_session(tab_session: &crate::session::TabSession) -> Self {
        let mut timeline = Timeline::new();
        for card_data in &tab_session.cards {
            let card = card_data.to_card();
            timeline.push_card(card);
        }
        if timeline.cards.is_empty() {
            return Self::new();
        }
        let sys_id = timeline.next_card_id();
        timeline.push_card(Card::System(SystemCard::new(
            sys_id,
            "Session restored.".to_string(),
        )));
        Self {
            timeline,
            input_bar: InputBar::new(),
            fullscreen_overlay: false,
            search_bar: SearchBar::new(),
        }
    }
}

pub struct AineerApp {
    tab_manager: TabManager,
    tab_states: Vec<(u64, TabState)>,
    pty_sender: mpsc::Sender<(u64, PtyEvent)>,
    pty_receiver: mpsc::Receiver<(u64, PtyEvent)>,
    terminal_theme: terminal::TerminalTheme,
    font_size: f32,
    diff_panel: DiffPanel,
    git_watcher: Option<GitWatcher>,
    git_status: Option<Arc<ui::git_diff::GitStatus>>,
    gateway_status: tokio::sync::watch::Receiver<gateway::GatewayStatus>,
    settings_panel: SettingsPanel,
    agent: AgentHandle,
    active_streams: Vec<ActiveStream>,
    _tokio_rt: Arc<tokio::runtime::Runtime>,
    slash_items: Vec<SlashMenuItem>,
    /// Fraction of central area allocated to the timeline (0.2 .. 0.85).
    split_fraction: f32,
    ssh_manager: SshManager,
    show_ssh_dialog: bool,
    ssh_draft: SshProfile,
    update_status: Arc<std::sync::Mutex<crate::updater::UpdateStatus>>,
}

impl AineerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::setup(&cc.egui_ctx);

        let (pty_sender, pty_receiver) = mpsc::channel();

        let mut tab_manager = TabManager::new();
        let session = crate::session::load_session();
        let saved_split = session.as_ref().map(|s| s.split_fraction).unwrap_or(0.6);

        let tab_states: Vec<(u64, TabState)> = if let Some(ref sess) = session {
            let mut states = Vec::new();
            for tab_session in &sess.tabs {
                tab_manager.create_tab(cc.egui_ctx.clone(), pty_sender.clone());
                let tab_id = tab_manager.active_tab_id().unwrap();
                if !tab_session.title.is_empty() {
                    tab_manager.set_title(tab_id, tab_session.title.clone());
                }
                states.push((tab_id, TabState::from_tab_session(tab_session)));
            }
            if states.is_empty() {
                tab_manager.create_tab(cc.egui_ctx.clone(), pty_sender.clone());
                let tab_id = tab_manager.active_tab_id().unwrap();
                states.push((tab_id, TabState::new()));
            } else if let Some(&(active_id, _)) = states.get(sess.active_tab_index) {
                tab_manager.set_active(active_id);
            }
            states
        } else {
            tab_manager.create_tab(cc.egui_ctx.clone(), pty_sender.clone());
            let tab_id = tab_manager.active_tab_id().unwrap();
            vec![(tab_id, TabState::new())]
        };

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let git_watcher = Some(GitWatcher::start(cwd));

        // Start tokio runtime for async tasks (Gateway, Agent, etc.)
        let rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime"),
        );

        // Start Gateway in background
        let gateway_config = gateway::GatewayConfig::default();
        let gateway = gateway::GatewayServer::new(gateway_config);
        let gateway_status = gateway.status_rx();
        rt.spawn(async move {
            if let Err(e) = gateway.start().await {
                tracing::error!("Gateway error: {e}");
            }
        });

        let agent = AgentHandle::spawn(&rt);

        let slash_items = engine::commands::slash_command_specs()
            .iter()
            .map(|spec| SlashMenuItem {
                name: spec.name.to_string(),
                summary: spec.summary.to_string(),
            })
            .collect();

        Self {
            tab_manager,
            tab_states,
            pty_sender,
            pty_receiver,
            terminal_theme: theme::aineer_terminal_theme(),
            font_size: 14.0,
            diff_panel: DiffPanel::new(),
            git_watcher,
            git_status: None,
            gateway_status,
            settings_panel: SettingsPanel::new(),
            agent,
            active_streams: Vec::new(),
            _tokio_rt: rt,
            slash_items,
            split_fraction: saved_split,
            ssh_manager: SshManager::new(),
            show_ssh_dialog: false,
            ssh_draft: SshProfile::default(),
            update_status: {
                let status = Arc::new(std::sync::Mutex::new(
                    crate::updater::UpdateStatus::Checking,
                ));
                let s = status.clone();
                std::thread::spawn(move || {
                    let result = crate::updater::check_for_update();
                    if let Ok(mut lock) = s.lock() {
                        *lock = result;
                    }
                });
                status
            },
        }
    }

    fn poll_git_status(&mut self) {
        if let Some(watcher) = &self.git_watcher {
            if let Some(status) = watcher.try_recv() {
                self.diff_panel.update_status(status.clone());
                self.git_status = Some(status);
            }
        }
    }

    fn tab_state_mut(&mut self, tab_id: u64) -> Option<&mut TabState> {
        self.tab_states
            .iter_mut()
            .find(|(id, _)| *id == tab_id)
            .map(|(_, state)| state)
    }

    fn ensure_tab_state(&mut self, tab_id: u64) {
        if !self.tab_states.iter().any(|(id, _)| *id == tab_id) {
            self.tab_states.push((tab_id, TabState::new()));
        }
    }

    fn process_pty_events(&mut self, ctx: &egui::Context) {
        while let Ok((tab_id, event)) = self.pty_receiver.try_recv() {
            match event {
                PtyEvent::Exit => {
                    self.tab_manager.remove_tab(tab_id);
                    self.tab_states.retain(|(id, _)| *id != tab_id);
                    if self.tab_manager.is_empty() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                PtyEvent::Title(title) => {
                    self.tab_manager.set_title(tab_id, title);
                }
                PtyEvent::ChildExit(exit_code) => {
                    // Snapshot styled output on child exit
                    if let Some(tab) = self.tab_manager.tab_mut(tab_id) {
                        let output = tab.backend.visible_text();
                        let styled: Vec<ui::cards::OutputLine> = tab
                            .backend
                            .visible_styled_lines(&self.terminal_theme)
                            .into_iter()
                            .map(|sl| ui::cards::OutputLine {
                                segments: sl
                                    .segments
                                    .into_iter()
                                    .map(|s| ui::cards::OutputSegment {
                                        text: s.text,
                                        fg: s.fg,
                                        bold: s.bold,
                                    })
                                    .collect(),
                            })
                            .collect();
                        let cwd = tab
                            .backend
                            .current_cwd()
                            .map(|p| p.to_string_lossy().to_string());
                        if let Some(state) = self.tab_state_mut(tab_id) {
                            if let Some(card) = state.timeline.last_shell_card_mut() {
                                if card.running {
                                    card.output_lines = output;
                                    card.styled_output = styled;
                                    if let Some(dir) = cwd {
                                        card.working_dir = dir;
                                    }
                                    card.running = false;
                                    card.exit_code = Some(exit_code);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_shell_submit(&mut self, command: String) {
        let Some(tab_id) = self.tab_manager.active_tab_id() else {
            return;
        };

        // Capture output from previous running ShellCard (snapshot terminal text + styled)
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            let output = tab.backend.visible_text();
            let styled = tab
                .backend
                .visible_styled_lines(&self.terminal_theme)
                .into_iter()
                .map(|sl| ui::cards::OutputLine {
                    segments: sl
                        .segments
                        .into_iter()
                        .map(|s| ui::cards::OutputSegment {
                            text: s.text,
                            fg: s.fg,
                            bold: s.bold,
                        })
                        .collect(),
                })
                .collect();
            let cwd = tab
                .backend
                .current_cwd()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "~".to_string());
            if let Some(state) = self.tab_state_mut(tab_id) {
                if let Some(prev_card) = state.timeline.last_shell_card_mut() {
                    if prev_card.running {
                        prev_card.output_lines = output;
                        prev_card.styled_output = styled;
                        prev_card.working_dir = cwd.clone();
                        prev_card.running = false;
                        prev_card.exit_code = None;
                    }
                }
            }
        }

        // Resolve CWD for new card
        let cwd = if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.backend
                .current_cwd()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "~".to_string())
        } else {
            "~".to_string()
        };

        // Create new ShellCard
        if let Some(state) = self.tab_state_mut(tab_id) {
            let card_id = state.timeline.next_card_id();
            state
                .timeline
                .push_card(Card::Shell(ShellCard::new(card_id, command.clone(), cwd)));
        }

        // Write command to PTY
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            let cmd_bytes = format!("{command}\n").into_bytes();
            tab.backend
                .process_command(BackendCommand::Write(cmd_bytes));
        }
    }

    fn handle_chat_submit(&mut self, text: String, refs: Vec<u64>) {
        let Some(tab_id) = self.tab_manager.active_tab_id() else {
            return;
        };

        let card_id;
        if let Some(state) = self.tab_state_mut(tab_id) {
            card_id = state.timeline.next_card_id();
            let mut card = ChatCard::new(card_id, text.clone(), refs.clone());
            card.response = "Thinking...".to_string();
            card.streaming = true;
            state.timeline.push_card(Card::Chat(card));
        } else {
            return;
        }

        // Collect context from referenced cards
        let context: Vec<String> = if let Some(state) = self.tab_state_mut(tab_id) {
            refs.iter()
                .filter_map(|ref_id| state.timeline.card_summary(*ref_id))
                .collect()
        } else {
            Vec::new()
        };

        // Start streaming from agent
        let (event_rx, approval_tx) = self.agent.send_chat(text, context);
        self.active_streams.push(ActiveStream {
            tab_id,
            card_id,
            event_rx,
            approval_tx,
        });
    }

    fn handle_timeline_action(&mut self, tab_id: u64, action: TimelineAction) {
        match action {
            TimelineAction::None => {}
            TimelineAction::ToolApprove {
                card_id,
                tool_use_id: _,
            } => {
                if let Some(stream) = self
                    .active_streams
                    .iter()
                    .find(|s| s.tab_id == tab_id && s.card_id == card_id)
                {
                    let _ = stream.approval_tx.try_send(ToolApproval::Allow);
                }
            }
            TimelineAction::ToolDeny {
                card_id,
                tool_use_id: _,
            } => {
                if let Some(stream) = self
                    .active_streams
                    .iter()
                    .find(|s| s.tab_id == tab_id && s.card_id == card_id)
                {
                    let _ = stream.approval_tx.try_send(ToolApproval::Deny);
                }
            }
            TimelineAction::ToolApproveAll { card_id } => {
                if let Some(stream) = self
                    .active_streams
                    .iter()
                    .find(|s| s.tab_id == tab_id && s.card_id == card_id)
                {
                    let _ = stream.approval_tx.try_send(ToolApproval::AllowAll);
                }
            }
            TimelineAction::AddRef { card_id } => {
                if let Some(state) = self.tab_state_mut(tab_id) {
                    state.input_bar.add_ref(card_id);
                }
            }
        }
    }

    /// Open a new tab connected to a remote host via system SSH.
    fn connect_ssh(&mut self, ctx: &egui::Context, profile: &SshProfile) {
        let ssh_args = profile.to_ssh_args();
        let settings = terminal::BackendSettings {
            shell: "ssh".to_string(),
            args: ssh_args,
            working_directory: None,
        };
        let id = self.tab_manager.create_tab_with_settings(
            ctx.clone(),
            self.pty_sender.clone(),
            settings,
        );
        if let Some(new_id) = id {
            self.ensure_tab_state(new_id);
            self.tab_manager
                .set_title(new_id, format!("{}@{}", profile.user, profile.host));
        }
    }

    fn handle_search_action(&mut self, tab_id: u64, action: SearchAction) {
        match action {
            SearchAction::None => {}
            SearchAction::QueryChanged(query) => {
                if let Some(tab) = self.tab_manager.tab_mut(tab_id) {
                    let valid = tab.backend.search_set_query(&query);
                    if let Some((_, state)) =
                        self.tab_states.iter_mut().find(|(id, _)| *id == tab_id)
                    {
                        state.search_bar.regex_valid = valid;
                    }
                }
            }
            SearchAction::FindNext => {
                if let Some(tab) = self.tab_manager.tab_mut(tab_id) {
                    tab.backend.search_find_next();
                }
            }
            SearchAction::FindPrev => {
                if let Some(tab) = self.tab_manager.tab_mut(tab_id) {
                    tab.backend.search_find_prev();
                }
            }
            SearchAction::Close => {
                if let Some(tab) = self.tab_manager.tab_mut(tab_id) {
                    tab.backend.search_clear();
                }
                if let Some((_, state)) = self.tab_states.iter_mut().find(|(id, _)| *id == tab_id) {
                    state.search_bar.close();
                }
            }
        }
    }

    fn poll_agent_streams(&mut self) {
        let mut completed = Vec::new();

        for (i, stream) in self.active_streams.iter().enumerate() {
            let mut events = Vec::new();
            while let Ok(event) = stream.event_rx.try_recv() {
                let is_done = matches!(event, AgentEvent::Done);
                events.push(event);
                if is_done {
                    completed.push(i);
                    break;
                }
            }

            if let Some(state) = self
                .tab_states
                .iter_mut()
                .find(|(id, _)| *id == stream.tab_id)
                .map(|(_, s)| s)
            {
                for event in events {
                    match event {
                        AgentEvent::TextDelta(text) => {
                            state.timeline.append_chat_response(stream.card_id, &text);
                        }
                        AgentEvent::Error(msg) => {
                            state.timeline.append_chat_response(
                                stream.card_id,
                                &format!("\n\n**Error:** {msg}"),
                            );
                        }
                        AgentEvent::Done => {
                            state.timeline.finish_chat_streaming(stream.card_id);
                        }
                        AgentEvent::ToolPending {
                            tool_use_id,
                            name,
                            input,
                        } => {
                            state.timeline.add_tool_pending(
                                stream.card_id,
                                tool_use_id,
                                name,
                                input,
                            );
                        }
                        AgentEvent::ToolRunning { tool_use_id } => {
                            state
                                .timeline
                                .set_tool_running(stream.card_id, &tool_use_id);
                        }
                        AgentEvent::ToolResult {
                            tool_use_id,
                            name: _,
                            output,
                            is_error,
                        } => {
                            state.timeline.set_tool_result(
                                stream.card_id,
                                &tool_use_id,
                                output,
                                is_error,
                            );
                        }
                    }
                }
            }
        }

        for i in completed.into_iter().rev() {
            self.active_streams.remove(i);
        }
    }

    fn show_tab_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        use ui::theme as t;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;

            ui.label(
                RichText::new(crate::branding::APP_NAME)
                    .color(t::ACCENT_LIGHT)
                    .size(13.0)
                    .strong(),
            );
            ui.add_space(6.0);

            let tabs = self.tab_manager.tab_ids_and_titles();
            let active_id = self.tab_manager.active_tab_id();

            for (id, title) in &tabs {
                let is_active = active_id == Some(*id);
                let btn = egui::Button::new(RichText::new(title).size(12.0).color(if is_active {
                    t::FG
                } else {
                    t::FG_DIM
                }))
                .fill(if is_active {
                    t::TAB_ACTIVE_BG
                } else {
                    t::TAB_INACTIVE_BG
                })
                .corner_radius(t::BUTTON_CORNER_RADIUS);

                let resp = ui.add(btn);
                if resp.clicked() {
                    self.tab_manager.set_active(*id);
                }
                if resp.middle_clicked() {
                    self.tab_manager.remove_tab(*id);
                    self.tab_states.retain(|(tid, _)| tid != id);
                    if self.tab_manager.is_empty() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    return;
                }
            }

            if ui
                .add(
                    egui::Button::new(RichText::new("+").color(t::FG_DIM))
                        .fill(t::TAB_INACTIVE_BG)
                        .corner_radius(t::BUTTON_CORNER_RADIUS),
                )
                .clicked()
            {
                self.tab_manager
                    .create_tab(ctx.clone(), self.pty_sender.clone());
                let new_id = self.tab_manager.active_tab_id().unwrap();
                self.ensure_tab_state(new_id);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("⚙").size(13.0).color(t::FG_SOFT))
                            .fill(t::BUTTON_BG)
                            .corner_radius(t::BUTTON_CORNER_RADIUS),
                    )
                    .clicked()
                {
                    self.settings_panel.toggle();
                }

                if ui
                    .add(
                        egui::Button::new(RichText::new("SSH").size(11.0).color(t::FG_SOFT))
                            .fill(t::BUTTON_BG)
                            .corner_radius(t::BUTTON_CORNER_RADIUS),
                    )
                    .on_hover_text("SSH Remote Connection")
                    .clicked()
                {
                    self.show_ssh_dialog = true;
                }

                let diff_label = if self.diff_panel.visible {
                    "◀ Diff"
                } else {
                    "Diff ▶"
                };
                if ui
                    .add(
                        egui::Button::new(RichText::new(diff_label).size(11.0).color(t::FG_SOFT))
                            .fill(t::BUTTON_BG)
                            .corner_radius(t::BUTTON_CORNER_RADIUS),
                    )
                    .clicked()
                {
                    self.diff_panel.toggle();
                }

                if let Some(ref status) = self.git_status {
                    if let Some(ref branch) = status.branch {
                        ui.add_space(4.0);
                        egui::Frame::new()
                            .fill(t::SURFACE)
                            .corner_radius(6.0)
                            .inner_margin(egui::Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("⎇ {branch}"))
                                        .size(11.0)
                                        .monospace()
                                        .color(t::FG_SOFT),
                                );
                            });
                    }
                }

                if let Ok(status) = self.update_status.lock() {
                    match &*status {
                        crate::updater::UpdateStatus::Available { tag, url, body } => {
                            let tag = tag.clone();
                            let url = url.clone();
                            let hover = if body.is_empty() {
                                format!("{tag} available — click to open")
                            } else {
                                let truncated = if body.len() > 200 {
                                    format!("{}…", &body[..200])
                                } else {
                                    body.clone()
                                };
                                format!("{tag} available\n\n{truncated}")
                            };
                            let resp = ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(format!("⬆ {tag}"))
                                            .size(10.0)
                                            .color(t::SUCCESS),
                                    )
                                    .fill(t::SURFACE)
                                    .corner_radius(4.0),
                                )
                                .on_hover_text(hover);
                            if resp.clicked() {
                                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                            }
                        }
                        crate::updater::UpdateStatus::Checking => {
                            ui.label(RichText::new("⟳").size(10.0).color(t::FG_MUTED))
                                .on_hover_text("Checking for updates...");
                        }
                        crate::updater::UpdateStatus::Error(msg) => {
                            ui.label(RichText::new("⚠").size(10.0).color(t::FG_MUTED))
                                .on_hover_text(format!("Update check failed: {msg}"));
                        }
                        _ => {}
                    }
                }

                let gw_status = *self.gateway_status.borrow();
                let (gw_color, gw_tip) = match gw_status {
                    gateway::GatewayStatus::Running => (t::SUCCESS, "Gateway: Running"),
                    gateway::GatewayStatus::Starting => (t::WARNING, "Gateway: Starting"),
                    gateway::GatewayStatus::Error => (t::ERROR, "Gateway: Error"),
                    gateway::GatewayStatus::Stopped => (t::FG_MUTED, "Gateway: Stopped"),
                };
                ui.label(RichText::new("●").color(gw_color).size(10.0))
                    .on_hover_text(gw_tip);
            });
        });
    }

    fn show_ssh_popup(&mut self, ctx: &egui::Context) {
        use ui::theme as t;

        if !self.show_ssh_dialog {
            return;
        }

        let mut open = self.show_ssh_dialog;
        let mut connect_profile: Option<SshProfile> = None;
        let mut remove_idx: Option<usize> = None;

        egui::Window::new("SSH Connection")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                if !self.ssh_manager.profiles.is_empty() {
                    ui.label(
                        RichText::new("Saved Profiles")
                            .strong()
                            .size(12.0)
                            .color(t::FG),
                    );
                    ui.add_space(4.0);

                    for (i, profile) in self.ssh_manager.profiles.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let label = if profile.name.is_empty() {
                                format!("{}@{}:{}", profile.user, profile.host, profile.port)
                            } else {
                                profile.name.clone()
                            };
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(&label).size(12.0).color(t::FG),
                                    )
                                    .fill(t::SURFACE)
                                    .corner_radius(4.0),
                                )
                                .clicked()
                            {
                                connect_profile = Some(profile.clone());
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("✕").size(11.0).color(t::FG_DIM),
                                    )
                                    .fill(egui::Color32::TRANSPARENT),
                                )
                                .on_hover_text("Remove profile")
                                .clicked()
                            {
                                remove_idx = Some(i);
                            }
                        });
                    }
                    ui.separator();
                }

                ui.label(
                    RichText::new("New Connection")
                        .strong()
                        .size(12.0)
                        .color(t::FG),
                );
                ui.add_space(4.0);

                egui::Grid::new("ssh_form")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Name:").size(11.0).color(t::FG_SOFT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ssh_draft.name)
                                .desired_width(200.0)
                                .hint_text("optional"),
                        );
                        ui.end_row();

                        ui.label(RichText::new("Host:").size(11.0).color(t::FG_SOFT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ssh_draft.host)
                                .desired_width(200.0),
                        );
                        ui.end_row();

                        ui.label(RichText::new("Port:").size(11.0).color(t::FG_SOFT));
                        let mut port_str = self.ssh_draft.port.to_string();
                        if ui
                            .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
                            .changed()
                        {
                            if let Ok(p) = port_str.parse::<u16>() {
                                self.ssh_draft.port = p;
                            }
                        }
                        ui.end_row();

                        ui.label(RichText::new("User:").size(11.0).color(t::FG_SOFT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ssh_draft.user)
                                .desired_width(200.0),
                        );
                        ui.end_row();

                        ui.label(RichText::new("Identity file:").size(11.0).color(t::FG_SOFT));
                        let mut key_path = self.ssh_draft.identity_file.clone().unwrap_or_default();
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut key_path)
                                    .desired_width(200.0)
                                    .hint_text("~/.ssh/id_rsa"),
                            )
                            .changed()
                        {
                            self.ssh_draft.identity_file = if key_path.is_empty() {
                                None
                            } else {
                                Some(key_path)
                            };
                        }
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let can_connect =
                        !self.ssh_draft.host.is_empty() && !self.ssh_draft.user.is_empty();

                    if ui
                        .add_enabled(
                            can_connect,
                            egui::Button::new(RichText::new("Connect").size(12.0).color(t::FG))
                                .fill(t::ACCENT),
                        )
                        .clicked()
                    {
                        connect_profile = Some(self.ssh_draft.clone());
                    }

                    if ui
                        .add_enabled(
                            can_connect,
                            egui::Button::new(
                                RichText::new("Save & Connect").size(12.0).color(t::FG),
                            )
                            .fill(t::ACCENT),
                        )
                        .clicked()
                    {
                        self.ssh_manager.add_profile(self.ssh_draft.clone());
                        connect_profile = Some(self.ssh_draft.clone());
                    }
                });
            });

        self.show_ssh_dialog = open;

        if let Some(idx) = remove_idx {
            self.ssh_manager.remove_profile(idx);
        }

        if let Some(profile) = connect_profile {
            self.connect_ssh(ctx, &profile);
            self.show_ssh_dialog = false;
            self.ssh_draft = SshProfile::default();
        }
    }
}

fn build_card_picker_items(timeline: &Timeline) -> Vec<CardPickerItem> {
    timeline
        .cards
        .iter()
        .map(|card| match card {
            Card::Shell(sc) => CardPickerItem {
                id: sc.id,
                kind: "Shell",
                label: sc.command.clone(),
            },
            Card::Chat(cc) => {
                let label = if cc.prompt.len() > 50 {
                    format!("{}…", &cc.prompt[..50])
                } else {
                    cc.prompt.clone()
                };
                CardPickerItem {
                    id: cc.id,
                    kind: "Chat",
                    label,
                }
            }
            Card::System(sc) => {
                let label = if sc.message.len() > 50 {
                    format!("{}…", &sc.message[..50])
                } else {
                    sc.message.clone()
                };
                CardPickerItem {
                    id: sc.id,
                    kind: "System",
                    label,
                }
            }
        })
        .collect()
}

impl eframe::App for AineerApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        let tab_titles: Vec<(u64, String)> = self.tab_manager.tab_ids_and_titles();
        let active_id = self.tab_manager.active_tab_id();

        let mut active_tab_index = 0usize;
        let tabs: Vec<crate::session::TabSession> = self
            .tab_states
            .iter()
            .enumerate()
            .map(|(i, (tid, state))| {
                if active_id == Some(*tid) {
                    active_tab_index = i;
                }
                let title = tab_titles
                    .iter()
                    .find(|(id, _)| id == tid)
                    .map(|(_, t)| t.clone())
                    .unwrap_or_default();
                let working_dir = self
                    .tab_manager
                    .tab(*tid)
                    .and_then(|tab| tab.backend.current_cwd())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                crate::session::TabSession {
                    title,
                    working_dir,
                    cards: state
                        .timeline
                        .cards
                        .iter()
                        .map(crate::session::CardData::from_card)
                        .collect(),
                }
            })
            .collect();

        let data = crate::session::SessionData {
            tabs,
            active_tab_index,
            split_fraction: self.split_fraction,
        };

        if let Err(e) = crate::session::save_session(&data) {
            tracing::warn!("Failed to save session: {e}");
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Global shortcuts
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(Key::T) {
                self.tab_manager
                    .create_tab(ctx.clone(), self.pty_sender.clone());
                let new_id = self.tab_manager.active_tab_id().unwrap();
                self.ensure_tab_state(new_id);
            }
            if i.modifiers.command && i.key_pressed(Key::W) {
                if let Some(id) = self.tab_manager.active_tab_id() {
                    self.tab_manager.remove_tab(id);
                    self.tab_states.retain(|(tid, _)| *tid != id);
                }
            }
        });
        // Ctrl+Shift+F / Cmd+Shift+F: toggle terminal search
        {
            let toggle_search =
                ctx.input(|i| i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::F));
            if toggle_search {
                if let Some(id) = self.tab_manager.active_tab_id() {
                    if let Some((_, state)) = self.tab_states.iter_mut().find(|(tid, _)| *tid == id)
                    {
                        state.search_bar.toggle();
                        if !state.search_bar.open {
                            if let Some(tab) = self.tab_manager.tab_mut(id) {
                                tab.backend.search_clear();
                            }
                        }
                    }
                }
            }
        }

        self.process_pty_events(ctx);
        self.poll_git_status();
        self.poll_agent_streams();

        if self.tab_manager.is_empty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let active_tab_id = self.tab_manager.active_tab_id().unwrap_or(0);

        // Check if we're in fullscreen overlay mode (interactive command like vim)
        let is_alternate_screen = self
            .tab_manager
            .active_tab_mut()
            .map(|t| {
                t.backend.sync();
                t.backend.is_alternate_screen()
            })
            .unwrap_or(false);

        // Update overlay state
        if let Some(state) = self.tab_state_mut(active_tab_id) {
            if is_alternate_screen && !state.fullscreen_overlay {
                state.fullscreen_overlay = true;
            } else if !is_alternate_screen && state.fullscreen_overlay {
                state.fullscreen_overlay = false;
            }
            state.input_bar.set_shell_paused(state.fullscreen_overlay);
        }

        let fullscreen_overlay = self
            .tab_state_mut(active_tab_id)
            .map(|s| s.fullscreen_overlay)
            .unwrap_or(false);

        use ui::theme as t;

        egui::TopBottomPanel::top("tab_bar")
            .exact_height(t::TOOLBAR_HEIGHT)
            .frame(
                egui::Frame::new()
                    .fill(t::BG_ELEVATED)
                    .stroke(egui::Stroke::new(1.0, t::BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
                self.show_tab_bar(ui, ctx);
            });

        if fullscreen_overlay {
            // Fullscreen terminal overlay for interactive commands (vim, htop, etc.)
            egui::TopBottomPanel::bottom("overlay_bar")
                .exact_height(28.0)
                .frame(
                    egui::Frame::new()
                        .fill(t::blend(t::BG, t::WARNING, 0.08))
                        .stroke(egui::Stroke::new(
                            1.0,
                            t::blend(t::BORDER_SUBTLE, t::WARNING, 0.3),
                        )),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Interactive mode — exit the program to return to cards")
                                .small()
                                .color(t::WARNING),
                        );
                    });
                });

            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(t::BG))
                .show(ctx, |ui| {
                    if let Some(tab) = self.tab_manager.active_tab_mut() {
                        let terminal = TerminalView::new(ui, &mut tab.backend)
                            .set_focus(true)
                            .set_theme(self.terminal_theme.clone())
                            .set_font(TerminalFont::new(FontSettings {
                                font_type: FontId::monospace(self.font_size),
                            }))
                            .set_size(Vec2::new(ui.available_width(), ui.available_height()));
                        ui.add(terminal);
                    }
                });
        } else {
            // Settings panel (rightmost)
            if self.settings_panel.open {
                egui::SidePanel::right("settings_panel")
                    .default_width(380.0)
                    .min_width(300.0)
                    .max_width(600.0)
                    .frame(
                        egui::Frame::new()
                            .fill(t::PANEL_BG)
                            .stroke(egui::Stroke::new(1.0, t::BORDER_SUBTLE))
                            .inner_margin(egui::Margin::same(12)),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.heading(RichText::new("Settings").size(15.0));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(RichText::new("✕").color(t::FG_DIM))
                                                .fill(t::SURFACE)
                                                .corner_radius(t::BUTTON_CORNER_RADIUS),
                                        )
                                        .clicked()
                                    {
                                        self.settings_panel.open = false;
                                    }
                                },
                            );
                        });
                        ui.separator();
                        self.settings_panel.show(ui);
                    });
            }

            // Diff panel (right side drawer)
            if self.diff_panel.visible {
                egui::SidePanel::right("diff_panel")
                    .default_width(320.0)
                    .min_width(240.0)
                    .max_width(600.0)
                    .frame(
                        egui::Frame::new()
                            .fill(t::PANEL_BG)
                            .stroke(egui::Stroke::new(1.0, t::BORDER_SUBTLE))
                            .inner_margin(egui::Margin::same(10)),
                    )
                    .show(ctx, |ui| {
                        ui.heading(RichText::new("Changes").size(14.0));
                        ui.separator();
                        let diff_action = self.diff_panel.show(ui);
                        if let ui::diff_panel::DiffAction::RevertHunk { file, hunk_idx } =
                            diff_action
                        {
                            if let Some(status) = &self.git_status {
                                if let Some(file_diff) = status.diffs.get(&file) {
                                    if let Some(hunk) = file_diff.hunks.get(hunk_idx) {
                                        if let Err(e) = ui::git_diff::revert_hunk(
                                            &status.repo_root,
                                            &file,
                                            hunk,
                                        ) {
                                            tracing::warn!("Revert hunk failed: {e}");
                                        }
                                    }
                                }
                            }
                        }
                    });
            }

            // Input bar at bottom
            egui::TopBottomPanel::bottom("input_bar")
                .frame(
                    egui::Frame::new()
                        .fill(t::BG_ELEVATED)
                        .stroke(egui::Stroke::new(1.0, t::BORDER_SUBTLE)),
                )
                .show(ctx, |ui| {
                    let tab_id = active_tab_id;
                    let settings_open = self.settings_panel.open;
                    if let Some(state) = self
                        .tab_states
                        .iter_mut()
                        .find(|(id, _)| *id == tab_id)
                        .map(|(_, s)| s)
                    {
                        let card_items = build_card_picker_items(&state.timeline);
                        state.input_bar.focus_enabled = !settings_open;
                        let action = state.input_bar.show(ui, &card_items, &self.slash_items);
                        match action {
                            SubmitAction::Shell(cmd) => {
                                // We need to defer this because we borrow self mutably above
                                ctx.memory_mut(|m| {
                                    m.data.insert_temp(egui::Id::new("pending_shell_cmd"), cmd);
                                });
                            }
                            SubmitAction::Chat { text, refs } => {
                                ctx.memory_mut(|m| {
                                    m.data.insert_temp(egui::Id::new("pending_chat_text"), text);
                                    m.data.insert_temp(egui::Id::new("pending_chat_refs"), refs);
                                });
                            }
                            SubmitAction::None => {}
                        }
                    }
                });

            // Process deferred actions
            let shell_cmd: Option<String> = ctx.memory_mut(|m| {
                m.data
                    .get_temp::<String>(egui::Id::new("pending_shell_cmd"))
            });
            if let Some(cmd) = shell_cmd {
                ctx.memory_mut(|m| {
                    m.data.remove::<String>(egui::Id::new("pending_shell_cmd"));
                });
                self.handle_shell_submit(cmd);
            }

            let chat_text: Option<String> = ctx.memory_mut(|m| {
                m.data
                    .get_temp::<String>(egui::Id::new("pending_chat_text"))
            });
            if let Some(text) = chat_text {
                let refs: Vec<u64> = ctx
                    .memory_mut(|m| {
                        m.data
                            .get_temp::<Vec<u64>>(egui::Id::new("pending_chat_refs"))
                    })
                    .unwrap_or_default();
                ctx.memory_mut(|m| {
                    m.data.remove::<String>(egui::Id::new("pending_chat_text"));
                    m.data
                        .remove::<Vec<u64>>(egui::Id::new("pending_chat_refs"));
                });
                self.handle_chat_submit(text, refs);
            }

            // Main content: timeline + live terminal
            let split_frac = self.split_fraction;
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(t::BG))
                .show(ctx, |ui| {
                    let available_height = ui.available_height();
                    let splitter_h = 6.0;
                    let min_timeline = 60.0;
                    let min_terminal = 60.0;
                    let timeline_height = (available_height * split_frac)
                        .clamp(min_timeline, available_height - min_terminal - splitter_h);

                    // Timeline area (top portion)
                    let timeline_action = ui
                        .allocate_ui(Vec2::new(ui.available_width(), timeline_height), |ui| {
                            if let Some(state) = self.tab_state_mut(active_tab_id) {
                                state.timeline.show(ui)
                            } else {
                                TimelineAction::None
                            }
                        })
                        .inner;
                    self.handle_timeline_action(active_tab_id, timeline_action);

                    // Draggable splitter
                    let splitter_rect = ui
                        .allocate_space(Vec2::new(ui.available_width(), splitter_h))
                        .1;
                    let splitter_resp = ui.interact(
                        splitter_rect,
                        egui::Id::new("timeline_terminal_splitter"),
                        egui::Sense::drag(),
                    );
                    ui.painter().hline(
                        splitter_rect.x_range(),
                        splitter_rect.center().y,
                        egui::Stroke::new(
                            1.0,
                            if splitter_resp.hovered() || splitter_resp.dragged() {
                                t::ACCENT
                            } else {
                                t::BORDER_SUBTLE
                            },
                        ),
                    );
                    if splitter_resp.hovered() || splitter_resp.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }
                    if splitter_resp.dragged() {
                        let dy = splitter_resp.drag_delta().y;
                        if dy != 0.0 {
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(egui::Id::new("splitter_drag_dy"), dy);
                            });
                        }
                    }

                    // Search bar (above terminal)
                    let search_action = if let Some((_, state)) = self
                        .tab_states
                        .iter_mut()
                        .find(|(id, _)| *id == active_tab_id)
                    {
                        state.search_bar.show(ui)
                    } else {
                        SearchAction::None
                    };
                    self.handle_search_action(active_tab_id, search_action);

                    // Live terminal view (bottom portion)
                    if let Some(tab) = self.tab_manager.active_tab_mut() {
                        let terminal = TerminalView::new(ui, &mut tab.backend)
                            .set_focus(false)
                            .set_theme(self.terminal_theme.clone())
                            .set_font(TerminalFont::new(FontSettings {
                                font_type: FontId::monospace(self.font_size),
                            }))
                            .set_size(Vec2::new(ui.available_width(), ui.available_height()));
                        ui.add(terminal);
                    }
                });

            // Apply splitter drag delta via egui memory
            {
                let drag_delta: f32 =
                    ctx.data_mut(|d| d.get_temp(egui::Id::new("splitter_drag_dy")).unwrap_or(0.0));
                if drag_delta != 0.0 {
                    ctx.data_mut(|d| {
                        d.remove::<f32>(egui::Id::new("splitter_drag_dy"));
                    });
                    let central_height = ctx.available_rect().height();
                    if central_height > 0.0 {
                        self.split_fraction =
                            (self.split_fraction + drag_delta / central_height).clamp(0.15, 0.85);
                    }
                }
            }
        }

        self.show_ssh_popup(ctx);
    }
}
