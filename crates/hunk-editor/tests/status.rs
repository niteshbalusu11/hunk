use hunk_editor::{EditorCommand, EditorState, Viewport};
use hunk_language::{LanguageId, ParseStatus};
use hunk_text::{BufferId, TextBuffer};

#[test]
fn editor_state_tracks_dirty_language_and_parse_status() {
    let buffer = TextBuffer::new(BufferId::new(3), "fn main() {}\n");
    let mut editor = EditorState::new(buffer);

    editor.apply(EditorCommand::SetViewport(Viewport {
        first_visible_line: 4,
        visible_line_count: 20,
        horizontal_offset: 2,
    }));
    editor.apply(EditorCommand::SetLanguage(Some(LanguageId::new(9))));
    editor.apply(EditorCommand::SetParseStatus(ParseStatus::Parsing));
    editor.apply(EditorCommand::ReplaceAll(
        "fn answer() -> i32 { 42 }\n".to_string(),
    ));

    let display = editor.display_snapshot();
    assert_eq!(display.viewport.first_visible_line, 4);
    assert!(display.dirty);
    assert_eq!(display.language_id, Some(LanguageId::new(9)));
    assert_eq!(display.parse_status, ParseStatus::Parsing);

    editor.apply(EditorCommand::MarkSaved);
    assert!(!editor.is_dirty());
}
