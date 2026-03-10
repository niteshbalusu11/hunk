pub(crate) fn ai_should_show_no_turns_empty_state(
    visible_row_count: usize,
    has_pending_thread_start: bool,
) -> bool {
    visible_row_count == 0 && !has_pending_thread_start
}
