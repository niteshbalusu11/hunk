#[path = "../src/app/ai_thread_catalog_scheduler.rs"]
mod ai_thread_catalog_scheduler;

use std::path::PathBuf;

use ai_thread_catalog_scheduler::AiWorkspaceCatalogLoadScheduler;

#[test]
fn scheduler_limits_initial_parallel_loads() {
    let mut scheduler = AiWorkspaceCatalogLoadScheduler::new(
        vec![
            PathBuf::from("/repo-a"),
            PathBuf::from("/repo-b"),
            PathBuf::from("/repo-c"),
        ],
        2,
    );

    let ready = scheduler.start_ready_loads();

    assert_eq!(
        ready,
        vec![PathBuf::from("/repo-a"), PathBuf::from("/repo-b")]
    );
    assert!(scheduler.has_in_flight_loads());
}

#[test]
fn scheduler_backfills_next_workspace_when_one_finishes() {
    let mut scheduler = AiWorkspaceCatalogLoadScheduler::new(
        vec![
            PathBuf::from("/repo-a"),
            PathBuf::from("/repo-b"),
            PathBuf::from("/repo-c"),
        ],
        2,
    );

    assert_eq!(
        scheduler.start_ready_loads(),
        vec![PathBuf::from("/repo-a"), PathBuf::from("/repo-b")]
    );

    let ready = scheduler.finish_one_and_start_ready_loads();

    assert_eq!(ready, vec![PathBuf::from("/repo-c")]);
    assert!(scheduler.has_in_flight_loads());
}

#[test]
fn scheduler_defaults_to_at_least_one_parallel_slot() {
    let mut scheduler = AiWorkspaceCatalogLoadScheduler::new(vec![PathBuf::from("/repo-a")], 0);

    let ready = scheduler.start_ready_loads();

    assert_eq!(ready, vec![PathBuf::from("/repo-a")]);
    assert!(scheduler.has_in_flight_loads());
    assert!(scheduler.finish_one_and_start_ready_loads().is_empty());
    assert!(!scheduler.has_in_flight_loads());
}
