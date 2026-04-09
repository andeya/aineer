use std::time::Instant;

pub type CardId = u64;

pub enum Card {
    Shell(ShellCard),
    Chat(ChatCard),
    System(SystemCard),
}

#[derive(Debug, Clone)]
pub enum ToolState {
    Pending,
    Running,
    Completed { output: String, is_error: bool },
    Denied,
}

#[derive(Debug, Clone)]
pub struct ToolTurn {
    pub tool_use_id: String,
    pub name: String,
    pub input: String,
    pub state: ToolState,
}

/// A styled text segment (fg color + optional bold).
#[derive(Debug, Clone)]
pub struct OutputSegment {
    pub text: String,
    pub fg: egui::Color32,
    pub bold: bool,
}

/// One styled output line.
#[derive(Debug, Clone)]
pub struct OutputLine {
    pub segments: Vec<OutputSegment>,
}

impl OutputLine {
    pub fn plain(text: String) -> Self {
        Self {
            segments: vec![OutputSegment {
                text,
                fg: crate::theme::FG_DIM,
                bold: false,
            }],
        }
    }

    pub fn text(&self) -> String {
        self.segments.iter().map(|s| s.text.as_str()).collect()
    }
}

pub struct ShellCard {
    pub id: CardId,
    pub command: String,
    pub output_lines: Vec<String>,
    pub styled_output: Vec<OutputLine>,
    pub exit_code: Option<i32>,
    pub working_dir: String,
    pub timestamp: Instant,
    pub collapsed: bool,
    pub running: bool,
}

pub struct ChatCard {
    pub id: CardId,
    pub prompt: String,
    pub context_refs: Vec<CardId>,
    pub response: String,
    pub streaming: bool,
    pub timestamp: Instant,
    pub tool_turns: Vec<ToolTurn>,
}

pub struct SystemCard {
    pub id: CardId,
    pub message: String,
    pub timestamp: Instant,
}

impl ShellCard {
    pub fn new(id: CardId, command: String, working_dir: String) -> Self {
        Self {
            id,
            command,
            output_lines: Vec::new(),
            styled_output: Vec::new(),
            exit_code: None,
            working_dir,
            timestamp: Instant::now(),
            collapsed: false,
            running: true,
        }
    }

    pub fn output_text(&self) -> String {
        self.output_lines.join("\n")
    }
}

impl ChatCard {
    pub fn new(id: CardId, prompt: String, context_refs: Vec<CardId>) -> Self {
        Self {
            id,
            prompt,
            context_refs,
            response: String::new(),
            streaming: false,
            timestamp: Instant::now(),
            tool_turns: Vec::new(),
        }
    }
}

impl SystemCard {
    pub fn new(id: CardId, message: String) -> Self {
        Self {
            id,
            message,
            timestamp: Instant::now(),
        }
    }
}

impl Card {
    pub fn id(&self) -> CardId {
        match self {
            Card::Shell(c) => c.id,
            Card::Chat(c) => c.id,
            Card::System(c) => c.id,
        }
    }
}
