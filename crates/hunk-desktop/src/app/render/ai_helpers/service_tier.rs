pub(crate) fn ai_service_tier_picker_label(
    selected: hunk_domain::state::AiServiceTierSelection,
) -> &'static str {
    match selected {
        hunk_domain::state::AiServiceTierSelection::Standard => "Standard",
        hunk_domain::state::AiServiceTierSelection::Fast => "Fast",
        hunk_domain::state::AiServiceTierSelection::Flex => "Flex",
    }
}

pub(crate) fn ai_service_tier_options()
-> &'static [(hunk_domain::state::AiServiceTierSelection, &'static str)] {
    &[
        (
            hunk_domain::state::AiServiceTierSelection::Standard,
            "Standard",
        ),
        (hunk_domain::state::AiServiceTierSelection::Fast, "Fast"),
        (hunk_domain::state::AiServiceTierSelection::Flex, "Flex"),
    ]
}
