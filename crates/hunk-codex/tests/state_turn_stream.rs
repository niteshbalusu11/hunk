use hunk_codex::state::AiState;
use hunk_codex::state::ItemStatus;
use hunk_codex::state::ReducerEvent;
use hunk_codex::state::ServerRequestDecision;
use hunk_codex::state::StreamEvent;
use hunk_codex::state::TurnStatus;

#[test]
fn turn_stream_reaches_correct_final_state() {
    let mut state = AiState::default();

    state.apply_stream_events(vec![
        event(
            200,
            "item-completed:i1",
            ReducerEvent::ItemCompleted {
                thread_id: "t1".to_string(),
                turn_id: "r1".to_string(),
                item_id: "i1".to_string(),
            },
        ),
        event(
            100,
            "thread-start:t1",
            ReducerEvent::ThreadStarted {
                thread_id: "t1".to_string(),
                cwd: "/repo".to_string(),
                title: Some("Feature Branch".to_string()),
                created_at: Some(50),
                updated_at: Some(100),
            },
        ),
        event(
            110,
            "turn-start:r1",
            ReducerEvent::TurnStarted {
                thread_id: "t1".to_string(),
                turn_id: "r1".to_string(),
            },
        ),
        event(
            120,
            "item-start:i1",
            ReducerEvent::ItemStarted {
                thread_id: "t1".to_string(),
                turn_id: "r1".to_string(),
                item_id: "i1".to_string(),
                kind: "commandExecution".to_string(),
            },
        ),
        event(
            140,
            "item-delta:i1:1",
            ReducerEvent::ItemDelta {
                thread_id: "t1".to_string(),
                turn_id: "r1".to_string(),
                item_id: "i1".to_string(),
                delta: "running".to_string(),
            },
        ),
        event(
            180,
            "server-request:approval-1",
            ReducerEvent::ServerRequestResolved {
                request_id: "approval-1".to_string(),
                item_id: Some("i1".to_string()),
                decision: ServerRequestDecision::Accept,
            },
        ),
        event(
            210,
            "turn-completed:r1",
            ReducerEvent::TurnCompleted {
                thread_id: "t1".to_string(),
                turn_id: "r1".to_string(),
            },
        ),
    ]);

    let turn = state
        .turns
        .values()
        .find(|turn| turn.thread_id == "t1" && turn.id == "r1")
        .expect("turn should exist");
    assert_eq!(turn.status, TurnStatus::Completed);

    let item = state
        .items
        .values()
        .find(|item| item.thread_id == "t1" && item.turn_id == "r1" && item.id == "i1")
        .expect("item should exist");
    assert_eq!(item.status, ItemStatus::Completed);
    assert_eq!(item.content, "running");

    let approval = state
        .server_requests
        .get("approval-1")
        .expect("approval should exist");
    assert_eq!(approval.decision, ServerRequestDecision::Accept);
}

fn event(sequence: u64, dedupe_key: &str, payload: ReducerEvent) -> StreamEvent {
    StreamEvent {
        sequence,
        dedupe_key: Some(dedupe_key.to_string()),
        payload,
    }
}
