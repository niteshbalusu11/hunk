impl DiffViewer {
    pub(super) fn toggle_sidebar_tree_action(
        &mut self,
        _: &ToggleSidebarTree,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_sidebar_tree(cx);
    }

    pub(super) fn toggle_sidebar_tree(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        if !self.sidebar_collapsed && self.repo_tree_nodes.is_empty() && !self.repo_tree_loading {
            self.request_repo_tree_reload(cx);
        }
        cx.notify();
    }

    pub(super) fn set_workspace_view_mode(&mut self, mode: WorkspaceViewMode, cx: &mut Context<Self>) {
        if self.workspace_view_mode == mode {
            if !self.sidebar_collapsed
                && mode != WorkspaceViewMode::JjWorkspace
                && self.repo_tree_nodes.is_empty()
                && !self.repo_tree_loading
            {
                self.request_repo_tree_reload(cx);
            }
            return;
        }

        self.workspace_view_mode = mode;

        if mode == WorkspaceViewMode::Files {
            self.right_pane_mode = RightPaneMode::FileEditor;
            let target_path = self
                .selected_path
                .clone()
                .or_else(|| self.files.first().map(|file| file.path.clone()));
            if let Some(path) = target_path {
                self.selected_path = Some(path.clone());
                self.selected_status = self
                    .files
                    .iter()
                    .find(|file| file.path == path)
                    .map(|file| file.status);
                self.request_file_editor_reload(path, cx);
            } else {
                self.clear_editor_state(cx);
            }
        } else if mode == WorkspaceViewMode::Diff {
            self.right_pane_mode = RightPaneMode::Diff;
        }

        if !self.sidebar_collapsed
            && mode != WorkspaceViewMode::JjWorkspace
            && self.repo_tree_nodes.is_empty()
            && !self.repo_tree_loading
        {
            self.request_repo_tree_reload(cx);
        }
        cx.notify();
    }

    pub(super) fn toggle_repo_tree_directory(&mut self, path: String, cx: &mut Context<Self>) {
        if self.repo_tree_expanded_dirs.contains(path.as_str()) {
            self.repo_tree_expanded_dirs.remove(path.as_str());
        } else {
            self.repo_tree_expanded_dirs.insert(path);
        }
        self.rebuild_repo_tree_rows();
        cx.notify();
    }

    pub(super) fn select_repo_tree_file(&mut self, path: String, cx: &mut Context<Self>) {
        self.selected_path = Some(path.clone());
        self.selected_status = self
            .files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.status);
        if self.workspace_view_mode == WorkspaceViewMode::Files {
            self.right_pane_mode = RightPaneMode::FileEditor;
            self.request_file_editor_reload(path, cx);
        } else {
            self.right_pane_mode = RightPaneMode::Diff;
            self.scroll_to_file_start(&path);
            self.last_visible_row_start = None;
            self.last_diff_scroll_offset = None;
            self.last_scroll_activity_at = Instant::now();
        }
        cx.notify();
    }

    pub(super) fn request_repo_tree_reload(&mut self, cx: &mut Context<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.repo_tree_nodes.clear();
            self.repo_tree_rows.clear();
            self.repo_tree_file_count = 0;
            self.repo_tree_folder_count = 0;
            self.repo_tree_expanded_dirs.clear();
            self.sidebar_repo_row_count = 0;
            self.sidebar_repo_list_state.reset(0);
            self.repo_tree_loading = false;
            self.repo_tree_error = None;
            self.repo_tree_last_reload = Instant::now();
            cx.notify();
            return;
        };

        if self.repo_tree_loading {
            return;
        }

        let epoch = self.next_repo_tree_epoch();
        let initial_load = self.repo_tree_nodes.is_empty();
        if initial_load {
            self.repo_tree_loading = true;
        }
        self.repo_tree_error = None;
        self.repo_tree_last_reload = std::time::Instant::now();

        self.repo_tree_task = cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { load_repo_tree(&repo_root) })
                .await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.repo_tree_epoch {
                        return;
                    }

                    self::apply_repo_tree_reload(this, result, cx);
                })
                .ok();
            }
        });
    }

    fn next_repo_tree_epoch(&mut self) -> usize {
        self.repo_tree_epoch = self.repo_tree_epoch.saturating_add(1);
        self.repo_tree_epoch
    }

    fn rebuild_repo_tree_rows(&mut self) {
        self.repo_tree_rows = flatten_repo_tree_rows(&self.repo_tree_nodes, &self.repo_tree_expanded_dirs);
    }
}

fn apply_repo_tree_reload(
    this: &mut DiffViewer,
    result: anyhow::Result<Vec<hunk::jj::RepoTreeEntry>>,
    cx: &mut Context<DiffViewer>,
) {
    this.repo_tree_loading = false;
    match result {
        Ok(entries) => {
            let (file_count, folder_count) = count_non_ignored_repo_tree_entries(&entries);
            this.repo_tree_nodes = build_repo_tree(&entries);
            this.repo_tree_file_count = file_count;
            this.repo_tree_folder_count = folder_count;
            this.repo_tree_error = None;
            this.repo_tree_expanded_dirs
                .retain(|path| repo_tree_has_directory(&this.repo_tree_nodes, path.as_str()));
            this.rebuild_repo_tree_rows();
            if let Some(path) = this.selected_path.clone()
                && this.workspace_view_mode == WorkspaceViewMode::Files
                && !repo_tree_contains_path(&this.repo_tree_nodes, path.as_str())
            {
                this.clear_editor_state(cx);
            }
        }
        Err(err) => {
            this.repo_tree_error = Some(format!("Failed to load repository tree: {err:#}"));
            this.repo_tree_nodes.clear();
            this.repo_tree_rows.clear();
            this.repo_tree_file_count = 0;
            this.repo_tree_folder_count = 0;
            this.repo_tree_expanded_dirs.clear();
            this.sidebar_repo_row_count = 0;
            this.sidebar_repo_list_state.reset(0);
        }
    }

    cx.notify();
}

fn repo_tree_contains_path(nodes: &[RepoTreeNode], path: &str) -> bool {
    for node in nodes {
        if node.path == path {
            return true;
        }
        if repo_tree_contains_path(&node.children, path) {
            return true;
        }
    }
    false
}

fn repo_tree_has_directory(nodes: &[RepoTreeNode], path: &str) -> bool {
    for node in nodes {
        if node.kind == RepoTreeNodeKind::Directory && node.path == path {
            return true;
        }
        if repo_tree_has_directory(&node.children, path) {
            return true;
        }
    }
    false
}
