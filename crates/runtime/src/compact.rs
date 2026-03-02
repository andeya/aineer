use crate::session::{ContentBlock, ConversationMessage, MessageRole, Session};

const COMPACT_CONTINUATION_PREAMBLE: &str =
    "This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\n";
const COMPACT_RECENT_MESSAGES_NOTE: &str = "Recent messages are preserved verbatim.";
const COMPACT_DIRECT_RESUME_INSTRUCTION: &str = "Continue the conversation from where it left off without asking the user any further questions. Resume directly — do not acknowledge the summary, do not recap what was happening, and do not preface with continuation text.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionConfig {
    pub preserve_recent_messages: usize,
    pub max_estimated_tokens: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            preserve_recent_messages: 4,
            max_estimated_tokens: 10_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionResult {
    pub summary: String,
    pub formatted_summary: String,
    pub compacted_session: Session,
    pub removed_message_count: usize,
}

#[must_use]
pub fn estimate_session_tokens(session: &Session) -> usize {
    session.messages.iter().map(estimate_message_tokens).sum()
}

#[must_use]
pub fn should_compact(session: &Session, config: CompactionConfig) -> bool {
    let start = compacted_summary_prefix_len(session);
    let compactable = &session.messages[start..];

    compactable.len() > config.preserve_recent_messages
        && compactable
            .iter()
            .map(estimate_message_tokens)
            .sum::<usize>()
            >= config.max_estimated_tokens
}

#[must_use]
pub fn format_compact_summary(summary: &str) -> String {
    let without_analysis = strip_tag_block(summary, "analysis");
    let formatted = if let Some(content) = extract_tag_block(&without_analysis, "summary") {
        without_analysis.replace(
            &format!("<summary>{content}</summary>"),
            &format!("Summary:\n{}", content.trim()),
        )
    } else {
        without_analysis
    };

    collapse_blank_lines(&formatted).trim().to_string()
}

#[must_use]
pub fn get_compact_continuation_message(
    summary: &str,
    suppress_follow_up_questions: bool,
    recent_messages_preserved: bool,
) -> String {
    let mut base = format!(
        "{COMPACT_CONTINUATION_PREAMBLE}{}",
        format_compact_summary(summary)
    );

    if recent_messages_preserved {
        base.push_str("\n\n");
        base.push_str(COMPACT_RECENT_MESSAGES_NOTE);
    }

    if suppress_follow_up_questions {
        base.push('\n');
        base.push_str(COMPACT_DIRECT_RESUME_INSTRUCTION);
    }

    base
}

#[must_use]
pub fn compact_session(session: &Session, config: CompactionConfig) -> CompactionResult {
    if !should_compact(session, config) {
        return CompactionResult {
            summary: String::new(),
            formatted_summary: String::new(),
            compacted_session: session.clone(),
            removed_message_count: 0,
        };
    }

    let existing_summary = session
        .messages
        .first()
