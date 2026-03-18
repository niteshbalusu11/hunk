use hunk_text::{BufferId, Selection, TextBuffer, TextPosition, TextRange};

#[test]
fn text_buffer_snapshot_tracks_version_and_shape() {
    let mut buffer = TextBuffer::new(BufferId::new(7), "alpha\nbeta\n");
    let initial = buffer.snapshot();

    assert_eq!(initial.buffer_id, BufferId::new(7));
    assert_eq!(initial.version, 0);
    assert_eq!(initial.line_count, 3);
    assert_eq!(initial.byte_len, "alpha\nbeta\n".len());

    buffer.set_text("gamma\n");

    let updated = buffer.snapshot();
    assert_eq!(updated.version, 1);
    assert_eq!(updated.line_count, 2);
    assert_eq!(updated.text, "gamma\n");
}

#[test]
fn selection_range_normalizes_backward_selection() {
    let selection = Selection::new(TextPosition::new(8, 3), TextPosition::new(2, 5));
    let range = selection.range();

    assert_eq!(
        range,
        TextRange::new(TextPosition::new(2, 5), TextPosition::new(8, 3))
    );
    assert!(!selection.is_caret());
}
