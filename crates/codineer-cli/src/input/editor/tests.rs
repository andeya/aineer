use super::super::session::{EditSession, EditorMode, KeyAction};
use super::super::suggestions::CommandEntry;
use super::super::text::{is_vim_toggle, selection_bounds};
use super::LineEditor;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn cmd(name: &str, desc: &str, has_args: bool) -> CommandEntry {
    CommandEntry {
        name: name.to_string(),
        description: desc.to_string(),
        has_args,
    }
}

#[test]
fn toggle_submission_detects_vim_command() {
    assert!(is_vim_toggle("/vim"));
    assert!(is_vim_toggle("  /vim  "));
    assert!(!is_vim_toggle("/help"));
    assert!(!is_vim_toggle("hello"));
}

#[test]
fn normal_mode_supports_motion_and_insert_transition() {
    let mut editor = LineEditor::new("> ", vec![]);
    editor.vim_enabled = true;
    let mut session = EditSession::new(true);
    session.text = "hello".to_string();
    session.cursor = session.text.len();
    let _ = session.handle_escape();

    editor.handle_char(&mut session, 'h');
    editor.handle_char(&mut session, 'i');
    editor.handle_char(&mut session, '!');

    assert_eq!(session.mode, EditorMode::Insert);
    assert_eq!(session.text, "hel!lo");
}

#[test]
fn yy_and_p_paste_yanked_line_after_current_line() {
    let mut editor = LineEditor::new("> ", vec![]);
    editor.vim_enabled = true;
    let mut session = EditSession::new(true);
    session.text = "alpha\nbeta\ngamma".to_string();
    session.cursor = 0;
    let _ = session.handle_escape();

    editor.handle_char(&mut session, 'y');
    editor.handle_char(&mut session, 'y');
    editor.handle_char(&mut session, 'p');

    assert_eq!(session.text, "alpha\nalpha\nbeta\ngamma");
}

#[test]
fn dd_and_p_paste_deleted_line_after_current_line() {
    let mut editor = LineEditor::new("> ", vec![]);
    editor.vim_enabled = true;
    let mut session = EditSession::new(true);
    session.text = "alpha\nbeta\ngamma".to_string();
    session.cursor = 0;
    let _ = session.handle_escape();

    editor.handle_char(&mut session, 'j');
    editor.handle_char(&mut session, 'd');
    editor.handle_char(&mut session, 'd');
    editor.handle_char(&mut session, 'p');

    assert_eq!(session.text, "alpha\ngamma\nbeta\n");
}

#[test]
fn visual_mode_tracks_selection_with_motions() {
    let mut editor = LineEditor::new("> ", vec![]);
    editor.vim_enabled = true;
    let mut session = EditSession::new(true);
    session.text = "alpha\nbeta".to_string();
    session.cursor = 0;
    let _ = session.handle_escape();

    editor.handle_char(&mut session, 'v');
    editor.handle_char(&mut session, 'j');
    editor.handle_char(&mut session, 'l');

    assert_eq!(session.mode, EditorMode::Visual);
    assert_eq!(
        selection_bounds(
            &session.text,
            session.visual_anchor.unwrap_or(0),
            session.cursor
        ),
        Some((0, 8))
    );
}

#[test]
fn command_mode_submits_colon_prefixed_input() {
    let mut editor = LineEditor::new("> ", vec![]);
    editor.vim_enabled = true;
    let mut session = EditSession::new(true);
    session.text = "draft".to_string();
    session.cursor = session.text.len();
    let _ = session.handle_escape();

    editor.handle_char(&mut session, ':');
    editor.handle_char(&mut session, 'q');
    editor.handle_char(&mut session, '!');
    let action = session.submit_or_toggle();

    assert_eq!(session.mode, EditorMode::Command);
    assert_eq!(session.command_buffer, ":q!");
    assert!(matches!(action, KeyAction::Submit(line) if line == ":q!"));
}

#[test]
fn push_history_ignores_blank_entries() {
    let mut editor = LineEditor::new("> ", vec![cmd("/help", "Show help", false)]);

    editor.push_history("   ");
    editor.push_history("/help");

    assert_eq!(editor.history, vec!["/help".to_string()]);
}

#[test]
fn slash_suggestions_filter_and_accept_tab() {
    let mut editor = LineEditor::new(
        "> ",
        vec![
            cmd("/help", "Show help", false),
            cmd("/hello", "Greet", true),
        ],
    );
    let mut session = EditSession::new(false);
    session.text = "/he".to_string();
    session.cursor = session.text.len();

    editor.update_suggestions(&session);
    assert!(editor.suggestion_state.is_some());
    assert_eq!(editor.suggestion_state.as_ref().unwrap().items.len(), 2);

    editor.accept_suggestion(&mut session);
    assert!(session.text.starts_with("/he"));
}

#[test]
fn slash_suggestions_navigate_down() {
    let mut editor = LineEditor::new(
        "> ",
        vec![
            cmd("/permissions", "Manage permissions", false),
            cmd("/plugin", "Manage plugins", false),
        ],
    );
    let mut session = EditSession::new(false);
    session.text = "/p".to_string();
    session.cursor = session.text.len();

    editor.update_suggestions(&session);
    assert!(editor.suggestion_state.is_some());

    let action = editor.handle_key_event(
        &mut session,
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
    );
    assert!(matches!(action, KeyAction::Continue));
    assert_eq!(editor.suggestion_state.as_ref().unwrap().selected, 1);

    // Verify update_suggestions preserves selection when text hasn't changed
    editor.update_suggestions(&session);
    assert_eq!(editor.suggestion_state.as_ref().unwrap().selected, 1);
}

#[test]
fn slash_suggestions_navigate_up_wraps() {
    let mut editor = LineEditor::new(
        "> ",
        vec![
            cmd("/help", "Show help", false),
            cmd("/hello", "Greet", true),
            cmd("/history", "Show history", false),
        ],
    );
    let mut session = EditSession::new(false);
    session.text = "/h".to_string();
    session.cursor = session.text.len();

    editor.update_suggestions(&session);
    assert_eq!(editor.suggestion_state.as_ref().unwrap().selected, 0);

    // Up from 0 wraps to last item
    editor.handle_key_event(&mut session, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    editor.update_suggestions(&session);
    assert_eq!(editor.suggestion_state.as_ref().unwrap().selected, 2);

    // Up again goes to 1
    editor.handle_key_event(&mut session, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    editor.update_suggestions(&session);
    assert_eq!(editor.suggestion_state.as_ref().unwrap().selected, 1);
}

#[test]
fn slash_suggestions_enter_accepts_selected() {
    let mut editor = LineEditor::new(
        "> ",
        vec![
            cmd("/permissions", "Manage permissions", false),
            cmd("/plugin", "Manage plugins", false),
        ],
    );
    let mut session = EditSession::new(false);
    session.text = "/p".to_string();
    session.cursor = session.text.len();

    editor.update_suggestions(&session);

    // Navigate to /plugin (index 1)
    editor.handle_key_event(
        &mut session,
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
    );
    editor.update_suggestions(&session);
    assert_eq!(editor.suggestion_state.as_ref().unwrap().selected, 1);

    // Press Enter to accept
    let action = editor.handle_key_event(
        &mut session,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );

    // /plugin has no args → execute_on_enter = true → Submit
    assert!(matches!(action, KeyAction::Submit(ref line) if line == "/plugin"));
}

#[test]
fn suggestions_selection_resets_when_items_change() {
    let mut editor = LineEditor::new(
        "> ",
        vec![
            cmd("/help", "Show help", false),
            cmd("/hello", "Greet", true),
            cmd("/history", "Show history", false),
        ],
    );
    let mut session = EditSession::new(false);
    session.text = "/h".to_string();
    session.cursor = session.text.len();

    editor.update_suggestions(&session);
    assert_eq!(editor.suggestion_state.as_ref().unwrap().items.len(), 3);

    // Select second item
    editor.handle_key_event(
        &mut session,
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
    );
    editor.update_suggestions(&session);
    assert_eq!(editor.suggestion_state.as_ref().unwrap().selected, 1);

    // Type 'e' → filter changes to "/he" → 2 items → selection resets
    editor.handle_key_event(
        &mut session,
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
    );
    editor.update_suggestions(&session);
    assert_eq!(editor.suggestion_state.as_ref().unwrap().items.len(), 2);
    assert_eq!(editor.suggestion_state.as_ref().unwrap().selected, 0);
}

#[test]
fn at_suggestions_enter_adds_trailing_space_for_directory() {
    use super::super::suggestions::{SuggestionItem, SuggestionState, SuggestionTrigger};

    let mut editor = LineEditor::new("> ", vec![]);

    let dir_item = || SuggestionItem {
        display: "+ src/".to_string(),
        description: String::new(),
        completion: "@src/".to_string(), // directory: no trailing space in raw completion
        execute_on_enter: false,
    };
    let at_trigger = || SuggestionTrigger::At {
        token_start: 0,
        token_len: 4,
    };

    // Tab: keeps raw completion (no space — allows drilling into src/)
    {
        let mut s = EditSession::new(false);
        s.text = "@src".to_string();
        s.cursor = 4;
        editor.suggestion_state = Some(SuggestionState {
            items: vec![dir_item()],
            selected: 0,
            trigger: at_trigger(),
        });
        editor.accept_suggestion(&mut s);
        assert_eq!(
            s.text, "@src/",
            "Tab should not add a space for directories"
        );
    }

    // Enter: always appends a trailing space
    {
        let mut s = EditSession::new(false);
        s.text = "@src".to_string();
        s.cursor = 4;
        editor.suggestion_state = Some(SuggestionState {
            items: vec![dir_item()],
            selected: 0,
            trigger: at_trigger(),
        });
        let action =
            editor.handle_key_event(&mut s, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(action, KeyAction::Continue));
        assert_eq!(s.text, "@src/ ", "Enter should append a trailing space");
    }
}

#[test]
fn ctrl_c_cancels_when_input_exists() {
    let mut editor = LineEditor::new("> ", vec![]);
    let mut session = EditSession::new(false);
    session.text = "draft".to_string();
    session.cursor = session.text.len();

    let action = editor.handle_key_event(
        &mut session,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );

    assert!(matches!(action, KeyAction::Cancel));
}
