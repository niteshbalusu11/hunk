pub(crate) fn ai_approval_policy_picker_label(full_access: bool) -> &'static str {
    if full_access {
        "Full access"
    } else {
        "Ask for approvals"
    }
}

pub(crate) fn ai_approval_policy_options() -> &'static [(bool, &'static str)] {
    &[(false, "Ask for approvals"), (true, "Full access")]
}
