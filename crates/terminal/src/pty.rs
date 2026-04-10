//! PTY management using alacritty_terminal.
//! Pure logic, no UI framework dependency.

pub struct PtyHandle {
    // Placeholder: will hold alacritty_terminal::Term and event loop
    _private: (),
}

impl PtyHandle {
    /// Create a new PTY with the given config.
    pub fn new(_config: &super::TerminalConfig) -> Result<Self, super::TerminalError> {
        // TODO: Initialize alacritty_terminal::Term + EventLoop
        Ok(Self { _private: () })
    }

    /// Write bytes to the PTY input (e.g., user keystrokes).
    pub fn write(&self, _data: &[u8]) -> Result<(), super::TerminalError> {
        // TODO: Forward to PTY master fd
        Ok(())
    }

    /// Take a snapshot of the current terminal content.
    pub fn snapshot(&self) -> super::TerminalContent {
        // TODO: Extract grid content from alacritty_terminal::Term
        super::TerminalContent {
            cells: Vec::new(),
            display_offset: 0,
            cursor: super::CursorPosition {
                line: 0,
                column: 0,
                shape: super::CursorShape::Block,
            },
            columns: 80,
            lines: 24,
            title: String::new(),
            mode: super::TerminalMode::Normal,
        }
    }
}
