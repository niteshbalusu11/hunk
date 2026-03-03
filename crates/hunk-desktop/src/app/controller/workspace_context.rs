impl DiffViewer {
    pub(super) fn sync_workspace_execution_context_from_state(&mut self) {
        self.workspace_execution_context = self.repo_root.as_ref().map(|workspace_root| {
            Self::build_workspace_execution_context(
                workspace_root.as_path(),
                self.graph_current_workspace_name.as_deref(),
                self.graph_active_bookmark.as_deref(),
                self.branch_name.as_str(),
                self.graph_working_copy_commit_id.as_deref(),
            )
        });
    }

    pub(super) fn workspace_context_comment_bookmark_key(
        context: &WorkspaceExecutionContext,
    ) -> String {
        // Keep current bookmark-scoped storage behavior for compatibility.
        context
            .active_bookmark
            .clone()
            .unwrap_or_else(|| "detached".to_string())
    }

    fn build_workspace_execution_context(
        workspace_root: &Path,
        workspace_name: Option<&str>,
        active_bookmark: Option<&str>,
        branch_name: &str,
        working_copy_commit_id: Option<&str>,
    ) -> WorkspaceExecutionContext {
        let workspace_name = workspace_name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("unknown")
            .to_string();
        let active_bookmark = Self::normalize_workspace_context_bookmark(active_bookmark, branch_name);
        let working_copy_commit_id = working_copy_commit_id
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(ToString::to_string);

        WorkspaceExecutionContext {
            workspace_name,
            workspace_root: workspace_root.to_path_buf(),
            active_bookmark,
            working_copy_commit_id,
        }
    }

    fn normalize_workspace_context_bookmark(
        active_bookmark: Option<&str>,
        branch_name: &str,
    ) -> Option<String> {
        let active = active_bookmark
            .map(str::trim)
            .filter(|name| !name.is_empty() && *name != "detached")
            .map(ToString::to_string);
        if active.is_some() {
            return active;
        }

        let branch_name = branch_name.trim();
        if branch_name.is_empty() || branch_name == "unknown" || branch_name == "detached" {
            None
        } else {
            Some(branch_name.to_string())
        }
    }
}

#[cfg(test)]
mod workspace_context_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn workspace_context_identity_stays_stable_for_same_inputs() {
        let left = DiffViewer::build_workspace_execution_context(
            Path::new("/tmp/repo"),
            Some("default"),
            Some("main"),
            "main",
            Some("abc123"),
        );
        let right = DiffViewer::build_workspace_execution_context(
            Path::new("/tmp/repo"),
            Some("default"),
            Some("main"),
            "main",
            Some("abc123"),
        );

        assert_eq!(left, right);
    }

    #[test]
    fn workspace_context_identity_changes_on_workspace_switch() {
        let before = DiffViewer::build_workspace_execution_context(
            Path::new("/tmp/repo-default"),
            Some("default"),
            Some("main"),
            "main",
            Some("abc123"),
        );
        let after = DiffViewer::build_workspace_execution_context(
            Path::new("/tmp/repo-ws2"),
            Some("ws2"),
            Some("main"),
            "main",
            Some("abc123"),
        );

        assert_ne!(before, after);
        assert_ne!(before.workspace_name, after.workspace_name);
        assert_ne!(before.workspace_root, after.workspace_root);
    }

    #[test]
    fn workspace_context_comment_scope_uses_detached_when_bookmark_missing() {
        let context = DiffViewer::build_workspace_execution_context(
            Path::new("/tmp/repo"),
            Some("default"),
            None,
            "unknown",
            Some("abc123"),
        );
        assert_eq!(
            DiffViewer::workspace_context_comment_bookmark_key(&context),
            "detached"
        );
    }

    #[test]
    fn workspace_context_comment_scope_uses_active_bookmark_when_available() {
        let context = DiffViewer::build_workspace_execution_context(
            Path::new("/tmp/repo"),
            Some("default"),
            Some("feature"),
            "main",
            Some("abc123"),
        );
        assert_eq!(
            DiffViewer::workspace_context_comment_bookmark_key(&context),
            "feature"
        );
    }
}
