use std::collections::BTreeSet;

use hunk_codex::state::ThreadSummary;

pub(crate) fn bookmark_first_sorted_threads(
    threads: impl IntoIterator<Item = ThreadSummary>,
    bookmarked_thread_ids: &BTreeSet<String>,
) -> Vec<ThreadSummary> {
    let mut threads = threads.into_iter().collect::<Vec<_>>();
    threads.sort_by(|left, right| {
        bookmarked_thread_ids
            .contains(right.id.as_str())
            .cmp(&bookmarked_thread_ids.contains(left.id.as_str()))
            .then_with(|| right.created_at.cmp(&left.created_at))
            .then_with(|| right.id.cmp(&left.id))
    });
    threads
}

pub(crate) fn thread_is_bookmarked(
    bookmarked_thread_ids: &BTreeSet<String>,
    thread_id: &str,
) -> bool {
    bookmarked_thread_ids.contains(thread_id)
}

pub(crate) fn visible_threads_contain_thread(
    visible_threads: &[ThreadSummary],
    thread_id: &str,
) -> bool {
    visible_threads.iter().any(|thread| thread.id == thread_id)
}
