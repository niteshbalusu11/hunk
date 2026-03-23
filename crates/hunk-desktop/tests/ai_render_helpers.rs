#[path = "../src/app/render/ai_helpers/empty_state.rs"]
mod empty_state;
#[path = "../src/app/render/ai_helpers/service_tier.rs"]
mod service_tier;

use empty_state::ai_should_show_no_turns_empty_state;
use service_tier::{ai_approval_policy_options, ai_approval_policy_picker_label};

#[test]
fn no_turns_empty_state_depends_on_visible_rows_after_filtering() {
    assert!(ai_should_show_no_turns_empty_state(0, false));
    assert!(!ai_should_show_no_turns_empty_state(0, true));
    assert!(!ai_should_show_no_turns_empty_state(1, false));
}

#[test]
fn approval_policy_picker_options_cover_both_modes() {
    assert_eq!(ai_approval_policy_picker_label(false), "Ask for approvals");
    assert_eq!(ai_approval_policy_picker_label(true), "Full access");
    assert_eq!(
        ai_approval_policy_options(),
        &[(false, "Ask for approvals"), (true, "Full access")]
    );
}
