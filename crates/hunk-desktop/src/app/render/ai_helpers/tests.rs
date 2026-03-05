#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod ai_helper_tests {
    use super::ai_account_summary;
    use super::ai_thread_status_text;
    use super::ai_item_display_label;
    use super::ai_rate_limit_summary;
    use super::ai_timeline_item_is_renderable;
    use super::ai_truncate_multiline_content;
    use hunk_codex::state::ItemDisplayMetadata;
    use hunk_codex::state::ItemStatus;
    use hunk_codex::state::ItemSummary;
    use hunk_codex::state::ThreadLifecycleStatus;

    fn rate_limit_window(
        used_percent: i32,
        window_duration_mins: Option<i64>,
        resets_at: Option<i64>,
    ) -> codex_app_server_protocol::RateLimitWindow {
        codex_app_server_protocol::RateLimitWindow {
            used_percent,
            window_duration_mins,
            resets_at,
        }
    }

    fn rate_limit_snapshot(
        primary: Option<codex_app_server_protocol::RateLimitWindow>,
        secondary: Option<codex_app_server_protocol::RateLimitWindow>,
    ) -> codex_app_server_protocol::RateLimitSnapshot {
        codex_app_server_protocol::RateLimitSnapshot {
            limit_id: Some("codex".to_string()),
            limit_name: Some("Codex".to_string()),
            primary,
            secondary,
            credits: None,
            plan_type: None,
        }
    }

    #[test]
    fn rate_limit_summary_reports_five_hour_and_weekly_windows() {
        let snapshot = rate_limit_snapshot(
            Some(rate_limit_window(42, Some(300), Some(1_700_000_000))),
            Some(rate_limit_window(19, Some(10_080), Some(1_700_300_000))),
        );

        let (five_hour, weekly) = ai_rate_limit_summary(Some(&snapshot), false);
        assert!(five_hour.contains("5h: 42% used"));
        assert!(weekly.contains("weekly: 19% used"));
        assert!(!five_hour.contains("1700000000"));
        assert!(!weekly.contains("1700300000"));
        assert!(five_hour.contains("UTC"));
        assert!(weekly.contains("UTC"));
    }

    #[test]
    fn rate_limit_summary_falls_back_to_unavailable_when_missing() {
        let (five_hour, weekly) = ai_rate_limit_summary(None, false);
        assert_eq!(five_hour, "5h: unavailable");
        assert_eq!(weekly, "weekly: unavailable");
    }

    #[test]
    fn rate_limit_summary_reports_loading_during_bootstrap() {
        let (five_hour, weekly) = ai_rate_limit_summary(None, true);
        assert_eq!(five_hour, "5h: loading");
        assert_eq!(weekly, "weekly: loading");
    }

    #[test]
    fn account_summary_reports_loading_while_bootstrapping() {
        let summary = ai_account_summary(None, false, true);
        assert_eq!(summary, "Loading account...");
    }

    #[test]
    fn rate_limit_summary_uses_primary_and_secondary_when_durations_are_unknown() {
        let snapshot = rate_limit_snapshot(
            Some(rate_limit_window(11, Some(60), Some(1_700_000_000))),
            Some(rate_limit_window(27, Some(120), Some(1_700_100_000))),
        );

        let (five_hour, weekly) = ai_rate_limit_summary(Some(&snapshot), false);
        assert!(five_hour.contains("5h: 11% used"));
        assert!(weekly.contains("weekly: 27% used"));
    }

    #[test]
    fn truncate_multiline_content_only_marks_overflow_when_needed() {
        let (single, single_truncated) = ai_truncate_multiline_content("line 1\nline 2", 3);
        assert_eq!(single, "line 1\nline 2");
        assert!(!single_truncated);

        let (truncated, is_truncated) =
            ai_truncate_multiline_content("line 1\nline 2\nline 3\nline 4", 3);
        assert_eq!(truncated, "line 1\nline 2\nline 3\n...");
        assert!(is_truncated);
    }

    #[test]
    fn item_display_label_maps_user_and_agent_labels() {
        assert_eq!(ai_item_display_label("userMessage"), "User");
        assert_eq!(ai_item_display_label("agentMessage"), "Agent");
        assert_eq!(ai_item_display_label("unknownKind"), "unknownKind");
    }

    #[test]
    fn thread_status_text_maps_lifecycle_states() {
        assert_eq!(ai_thread_status_text(ThreadLifecycleStatus::Active), "active");
        assert_eq!(ai_thread_status_text(ThreadLifecycleStatus::Idle), "idle");
        assert_eq!(
            ai_thread_status_text(ThreadLifecycleStatus::NotLoaded),
            "not loaded"
        );
    }

    #[test]
    fn timeline_item_renderability_hides_empty_reasoning_without_metadata() {
        let reasoning = ItemSummary {
            id: "item-1".to_string(),
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            kind: "reasoning".to_string(),
            status: ItemStatus::Completed,
            content: "   ".to_string(),
            display_metadata: None,
            last_sequence: 1,
        };
        assert!(!ai_timeline_item_is_renderable(&reasoning));

        let reasoning_with_metadata = ItemSummary {
            display_metadata: Some(ItemDisplayMetadata {
                summary: Some("Thinking".to_string()),
                details_json: None,
            }),
            ..reasoning
        };
        assert!(ai_timeline_item_is_renderable(&reasoning_with_metadata));
    }
}
