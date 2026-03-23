#[path = "../src/app/ai_bookmarks.rs"]
mod ai_bookmarks;

use std::collections::BTreeSet;

use hunk_codex::state::{ThreadLifecycleStatus, ThreadSummary};

fn thread_summary(id: &str, created_at: i64) -> ThreadSummary {
    ThreadSummary {
        id: id.to_string(),
        cwd: "/repo".to_string(),
        title: Some(id.to_string()),
        status: ThreadLifecycleStatus::Idle,
        created_at,
        updated_at: created_at,
        last_sequence: 1,
    }
}

#[test]
fn bookmark_first_sorted_threads_prioritize_bookmarks_before_created_at() {
    let bookmarked = BTreeSet::from(["thread-1".to_string(), "thread-3".to_string()]);
    let threads = vec![
        thread_summary("thread-2", 30),
        thread_summary("thread-1", 10),
        thread_summary("thread-4", 20),
        thread_summary("thread-3", 5),
    ];

    let sorted = ai_bookmarks::bookmark_first_sorted_threads(threads, &bookmarked);
    let ids = sorted
        .into_iter()
        .map(|thread| thread.id)
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["thread-1", "thread-3", "thread-2", "thread-4"]);
}

#[test]
fn thread_is_bookmarked_matches_membership() {
    let bookmarked = BTreeSet::from(["thread-1".to_string()]);

    assert!(ai_bookmarks::thread_is_bookmarked(&bookmarked, "thread-1"));
    assert!(!ai_bookmarks::thread_is_bookmarked(&bookmarked, "thread-2"));
}

#[test]
fn visible_threads_contain_thread_matches_sidebar_rows() {
    let visible_threads = vec![
        thread_summary("thread-1", 10),
        thread_summary("thread-2", 20),
    ];

    assert!(ai_bookmarks::visible_threads_contain_thread(
        &visible_threads,
        "thread-2"
    ));
    assert!(!ai_bookmarks::visible_threads_contain_thread(
        &visible_threads,
        "thread-3"
    ));
}
