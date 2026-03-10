#[path = "../src/app/render/ai_helpers/empty_state.rs"]
mod empty_state;
#[path = "../src/app/render/ai_helpers/service_tier.rs"]
mod service_tier;

use empty_state::ai_should_show_no_turns_empty_state;
use hunk_domain::state::AiServiceTierSelection;
use service_tier::{ai_service_tier_options, ai_service_tier_picker_label};

#[test]
fn no_turns_empty_state_depends_on_visible_rows_after_filtering() {
    assert!(ai_should_show_no_turns_empty_state(0, false));
    assert!(!ai_should_show_no_turns_empty_state(0, true));
    assert!(!ai_should_show_no_turns_empty_state(1, false));
}

#[test]
fn service_tier_picker_options_include_flex() {
    assert_eq!(
        ai_service_tier_picker_label(AiServiceTierSelection::Standard),
        "Standard"
    );
    assert_eq!(
        ai_service_tier_picker_label(AiServiceTierSelection::Fast),
        "Fast"
    );
    assert_eq!(
        ai_service_tier_picker_label(AiServiceTierSelection::Flex),
        "Flex"
    );
    assert_eq!(
        ai_service_tier_options(),
        &[
            (AiServiceTierSelection::Standard, "Standard"),
            (AiServiceTierSelection::Fast, "Fast"),
            (AiServiceTierSelection::Flex, "Flex"),
        ]
    );
}
