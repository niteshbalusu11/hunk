#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkspaceViewMode {
    Files,
    Diff,
    JjWorkspace,
    Ai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkspaceSwitchAction {
    Files,
    Review,
    Git,
    Ai,
}

impl WorkspaceViewMode {
    pub(super) const fn supports_sidebar_tree(self) -> bool {
        matches!(self, Self::Files | Self::Diff)
    }

    pub(super) const fn supports_diff_stream(self) -> bool {
        matches!(self, Self::Files | Self::Diff)
    }
}

impl WorkspaceSwitchAction {
    pub(super) const fn target_mode(self) -> WorkspaceViewMode {
        match self {
            Self::Files => WorkspaceViewMode::Files,
            Self::Review => WorkspaceViewMode::Diff,
            Self::Git => WorkspaceViewMode::JjWorkspace,
            Self::Ai => WorkspaceViewMode::Ai,
        }
    }
}
