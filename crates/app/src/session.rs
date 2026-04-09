use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use ui::cards::{Card, ChatCard, OutputLine, OutputSegment, ShellCard, SystemCard};

const SESSION_DIR: &str = ".aineer/sessions";
const SESSION_FILE: &str = "last_session.json";

fn session_dir() -> PathBuf {
    dirs_home().join(SESSION_DIR)
}

fn session_file() -> PathBuf {
    session_dir().join(SESSION_FILE)
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub tabs: Vec<TabSession>,
    pub active_tab_index: usize,
    #[serde(default = "default_split_fraction")]
    pub split_fraction: f32,
}

fn default_split_fraction() -> f32 {
    0.6
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabSession {
    pub title: String,
    pub working_dir: String,
    pub cards: Vec<CardData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyledSegmentData {
    pub text: String,
    pub fg: [u8; 4],
    pub bold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyledLineData {
    pub segments: Vec<StyledSegmentData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CardData {
    Shell {
        id: u64,
        command: String,
        output_lines: Vec<String>,
        #[serde(default)]
        styled_output: Vec<StyledLineData>,
        exit_code: Option<i32>,
        cwd: String,
    },
    Chat {
        id: u64,
        prompt: String,
        response: String,
        context_refs: Vec<u64>,
    },
    System {
        id: u64,
        message: String,
    },
}

impl CardData {
    pub fn from_card(card: &Card) -> Self {
        match card {
            Card::Shell(s) => CardData::Shell {
                id: s.id,
                command: s.command.clone(),
                output_lines: s.output_lines.clone(),
                styled_output: s
                    .styled_output
                    .iter()
                    .map(|line| StyledLineData {
                        segments: line
                            .segments
                            .iter()
                            .map(|seg| StyledSegmentData {
                                text: seg.text.clone(),
                                fg: seg.fg.to_array(),
                                bold: seg.bold,
                            })
                            .collect(),
                    })
                    .collect(),
                exit_code: s.exit_code,
                cwd: s.working_dir.clone(),
            },
            Card::Chat(c) => CardData::Chat {
                id: c.id,
                prompt: c.prompt.clone(),
                response: c.response.clone(),
                context_refs: c.context_refs.clone(),
            },
            Card::System(s) => CardData::System {
                id: s.id,
                message: s.message.clone(),
            },
        }
    }

    pub fn to_card(&self) -> Card {
        match self {
            CardData::Shell {
                id,
                command,
                output_lines,
                styled_output,
                exit_code,
                cwd,
            } => {
                let mut card = ShellCard::new(*id, command.clone(), cwd.clone());
                card.output_lines = output_lines.clone();
                card.styled_output = styled_output
                    .iter()
                    .map(|line| OutputLine {
                        segments: line
                            .segments
                            .iter()
                            .map(|seg| OutputSegment {
                                text: seg.text.clone(),
                                fg: egui::Color32::from_rgba_premultiplied(
                                    seg.fg[0], seg.fg[1], seg.fg[2], seg.fg[3],
                                ),
                                bold: seg.bold,
                            })
                            .collect(),
                    })
                    .collect();
                card.exit_code = *exit_code;
                card.running = false;
                Card::Shell(card)
            }
            CardData::Chat {
                id,
                prompt,
                response,
                context_refs,
            } => {
                let mut card = ChatCard::new(*id, prompt.clone(), context_refs.clone());
                card.response = response.clone();
                Card::Chat(card)
            }
            CardData::System { id, message } => Card::System(SystemCard::new(*id, message.clone())),
        }
    }
}

pub fn save_session(data: &SessionData) -> Result<(), String> {
    let dir = session_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create session dir: {e}"))?;

    let json =
        serde_json::to_string_pretty(data).map_err(|e| format!("Failed to serialize: {e}"))?;

    std::fs::write(session_file(), json).map_err(|e| format!("Failed to write session: {e}"))
}

pub fn load_session() -> Option<SessionData> {
    let path = session_file();
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}
