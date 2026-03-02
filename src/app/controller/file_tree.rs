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
        if !self.sidebar_collapsed && self.repo_tree.nodes.is_empty() && !self.repo_tree.loading {
            self.request_repo_tree_reload(cx);
        }
        cx.notify();
    }

    pub(super) fn switch_to_files_view_action(
        &mut self,
        _: &SwitchToFilesView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_handle.focus(window);
        self.set_workspace_view_mode(WorkspaceViewMode::Files, cx);
    }

    pub(super) fn switch_to_review_view_action(
        &mut self,
        _: &SwitchToReviewView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_handle.focus(window);
        self.set_workspace_view_mode(WorkspaceViewMode::Diff, cx);
    }

    pub(super) fn switch_to_graph_view_action(
        &mut self,
        _: &SwitchToGraphView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_handle.focus(window);
        self.set_workspace_view_mode(WorkspaceViewMode::JjWorkspace, cx);
    }

    pub(super) fn set_workspace_view_mode(&mut self, mode: WorkspaceViewMode, cx: &mut Context<Self>) {
        let previous_mode = self.workspace_view_mode;
        if previous_mode == mode {
            if !self.sidebar_collapsed
                && mode != WorkspaceViewMode::JjWorkspace
                && self.repo_tree.nodes.is_empty()
                && !self.repo_tree.loading
            {
                self.request_repo_tree_reload(cx);
            }
            return;
        }

        if previous_mode == WorkspaceViewMode::Files {
            self.capture_sidebar_repo_scroll_anchor();
            if self.repo_tree.full_cache.is_some() {
                self.sync_full_repo_tree_cache_from_current();
            }
        }

        self.workspace_view_mode = mode;

        if mode == WorkspaceViewMode::Files {
            if previous_mode == WorkspaceViewMode::Diff || self.repo_tree.changed_only {
                if !self.restore_full_repo_tree_from_cache() && !self.repo_tree.loading {
                    self.request_repo_tree_reload(cx);
                }
            } else if self.repo_tree.nodes.is_empty() && !self.repo_tree.loading {
                self.request_repo_tree_reload(cx);
            }

            let target_path = self.editor_path.clone().or_else(|| self.selected_path.clone()).or_else(|| {
                self.files
                    .iter()
                    .find(|file| file.status != FileStatus::Deleted)
                    .map(|file| file.path.clone())
            });
            if let Some(path) = target_path {
                let editor_already_open = self.editor_path.as_deref() == Some(path.as_str())
                    && !self.editor_loading
                    && self.editor_error.is_none();
                if !editor_already_open
                    && self.prevent_unsaved_editor_discard(Some(path.as_str()), cx)
                {
                    return;
                }
                self.selected_path = Some(path.clone());
                self.selected_status = self.status_for_path(path.as_str());
                if !editor_already_open {
                    self.request_file_editor_reload(path, cx);
                }
            } else {
                if self.prevent_unsaved_editor_discard(None, cx) {
                    return;
                }
                self.selected_path = None;
                self.selected_status = None;
                self.clear_editor_state(cx);
            }
        } else if mode == WorkspaceViewMode::Diff {
            let selected_in_changed_files = self
                .selected_path
                .as_ref()
                .is_some_and(|selected| self.files.iter().any(|file| &file.path == selected));
            if !selected_in_changed_files {
                self.selected_path = self.files.first().map(|file| file.path.clone());
                self.selected_status = self
                    .selected_path
                    .as_deref()
                    .and_then(|selected| self.status_for_path(selected));
            }
            self.request_repo_tree_reload(cx);
            self.scroll_selected_after_reload = true;
            self.request_selected_diff_reload(cx);
        }
        cx.notify();
    }

    pub(super) fn toggle_repo_tree_directory(&mut self, path: String, cx: &mut Context<Self>) {
        if self.repo_tree.expanded_dirs.contains(path.as_str()) {
            self.repo_tree.expanded_dirs.remove(path.as_str());
        } else {
            self.repo_tree.expanded_dirs.insert(path);
        }
        self.rebuild_repo_tree_rows();
        cx.notify();
    }

    pub(super) fn select_repo_tree_file(&mut self, path: String, cx: &mut Context<Self>) {
        if self.workspace_view_mode == WorkspaceViewMode::Files
            && self.prevent_unsaved_editor_discard(Some(path.as_str()), cx)
        {
            return;
        }

        self.selected_path = Some(path.clone());
        self.selected_status = self.status_for_path(path.as_str());
        if self.workspace_view_mode == WorkspaceViewMode::Files {
            self.request_file_editor_reload(path, cx);
        } else {
            self.scroll_to_file_start(&path);
            self.last_visible_row_start = None;
            self.last_diff_scroll_offset = None;
            self.last_scroll_activity_at = Instant::now();
        }
        cx.notify();
    }

    pub(super) fn request_repo_tree_reload(&mut self, cx: &mut Context<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.repo_tree.nodes.clear();
            self.repo_tree.rows.clear();
            self.repo_tree.file_count = 0;
            self.repo_tree.folder_count = 0;
            self.repo_tree.expanded_dirs.clear();
            self.repo_tree.scroll_anchor_path = None;
            self.repo_tree.row_count = 0;
            self.repo_tree.list_state.reset(0);
            self.clear_full_repo_tree_cache();
            self.repo_tree.loading = false;
            self.repo_tree.reload_pending = false;
            self.repo_tree.error = None;
            self.repo_tree.changed_only = false;
            self.repo_tree.last_reload = Instant::now();
            cx.notify();
            return;
        };

        if self.workspace_view_mode == WorkspaceViewMode::Diff {
            self.next_repo_tree_epoch();
            self.repo_tree.task = Task::ready(());
            self.repo_tree.loading = false;
            self.repo_tree.reload_pending = false;
            self.repo_tree.error = None;
            self.repo_tree.changed_only = true;
            self.repo_tree.last_reload = std::time::Instant::now();
            self.rebuild_repo_tree_for_changed_files();
            cx.notify();
            return;
        }

        if self.repo_tree.loading {
            self.repo_tree.reload_pending = true;
            return;
        }

        let epoch = self.next_repo_tree_epoch();
        self.repo_tree.loading = true;
        self.repo_tree.reload_pending = false;
        self.repo_tree.error = None;
        self.repo_tree.changed_only = false;
        self.repo_tree.last_reload = std::time::Instant::now();

        self.repo_tree.task = cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { load_repo_tree(&repo_root) })
                .await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.repo_tree.epoch {
                        return;
                    }

                    self::apply_repo_tree_reload(this, result, cx);
                })
                .ok();
            }
        });
    }

    fn next_repo_tree_epoch(&mut self) -> usize {
        self.repo_tree.epoch = self.repo_tree.epoch.saturating_add(1);
        self.repo_tree.epoch
    }

    fn rebuild_repo_tree_rows(&mut self) {
        self.capture_sidebar_repo_scroll_anchor();
        self.repo_tree.rows = flatten_repo_tree_rows(&self.repo_tree.nodes, &self.repo_tree.expanded_dirs);
    }

    fn rebuild_repo_tree_for_changed_files(&mut self) {
        self.repo_tree.nodes = build_changed_files_tree(&self.files);
        self.repo_tree.file_count = count_repo_tree_kind(&self.repo_tree.nodes, RepoTreeNodeKind::File);
        self.repo_tree.folder_count =
            count_repo_tree_kind(&self.repo_tree.nodes, RepoTreeNodeKind::Directory);
        self.repo_tree.expanded_dirs.clear();
        self.rebuild_repo_tree_rows();
    }

    fn sync_full_repo_tree_cache_from_current(&mut self) {
        self.repo_tree.full_cache = Some(RepoTreeCacheState {
            nodes: self.repo_tree.nodes.clone(),
            file_count: self.repo_tree.file_count,
            folder_count: self.repo_tree.folder_count,
            expanded_dirs: self.repo_tree.expanded_dirs.clone(),
            error: self.repo_tree.error.clone(),
            scroll_anchor_path: self.repo_tree.scroll_anchor_path.clone(),
            fingerprint: self.last_snapshot_fingerprint.clone(),
        });
    }

    fn restore_full_repo_tree_from_cache(&mut self) -> bool {
        let Some(cache) = self.repo_tree.full_cache.as_ref() else {
            return false;
        };
        if cache.fingerprint != self.last_snapshot_fingerprint {
            return false;
        }

        self.repo_tree.nodes = cache.nodes.clone();
        self.repo_tree.file_count = cache.file_count;
        self.repo_tree.folder_count = cache.folder_count;
        self.repo_tree.expanded_dirs = cache.expanded_dirs.clone();
        self.repo_tree.error = cache.error.clone();
        self.repo_tree.rows = flatten_repo_tree_rows(&self.repo_tree.nodes, &self.repo_tree.expanded_dirs);
        self.repo_tree.scroll_anchor_path = cache.scroll_anchor_path.clone();
        self.repo_tree.row_count = 0;
        self.repo_tree.loading = false;
        self.repo_tree.reload_pending = false;
        self.repo_tree.changed_only = false;
        true
    }

    pub(super) fn clear_full_repo_tree_cache(&mut self) {
        self.repo_tree.full_cache = None;
    }
}

fn apply_repo_tree_reload(
    this: &mut DiffViewer,
    result: anyhow::Result<Vec<hunk::jj::RepoTreeEntry>>,
    cx: &mut Context<DiffViewer>,
) {
    this.repo_tree.loading = false;
    match result {
        Ok(entries) => {
            let (file_count, folder_count) = count_non_ignored_repo_tree_entries(&entries);
            this.repo_tree.nodes = build_repo_tree(&entries);
            this.repo_tree.file_count = file_count;
            this.repo_tree.folder_count = folder_count;
            this.repo_tree.error = None;
            this.repo_tree.changed_only = false;
            this.repo_tree.expanded_dirs
                .retain(|path| repo_tree_has_directory(&this.repo_tree.nodes, path.as_str()));
            this.rebuild_repo_tree_rows();
            if let Some(path) = this.selected_path.clone()
                && this.workspace_view_mode == WorkspaceViewMode::Files
                && !repo_tree_contains_path(&this.repo_tree.nodes, path.as_str())
                && !this.prevent_unsaved_editor_discard(None, cx)
            {
                this.clear_editor_state(cx);
                this.selected_path = None;
                this.selected_status = None;
            }
            if this.workspace_view_mode != WorkspaceViewMode::Diff {
                this.sync_full_repo_tree_cache_from_current();
            }
        }
        Err(err) => {
            this.repo_tree.error = Some(format!("Failed to load repository tree: {err:#}"));
            this.repo_tree.nodes.clear();
            this.repo_tree.rows.clear();
            this.repo_tree.file_count = 0;
            this.repo_tree.folder_count = 0;
            this.repo_tree.expanded_dirs.clear();
            this.repo_tree.changed_only = false;
            this.repo_tree.scroll_anchor_path = None;
            this.repo_tree.row_count = 0;
            this.repo_tree.list_state.reset(0);
        }
    }

    if this.repo_tree.reload_pending {
        this.repo_tree.reload_pending = false;
        this.request_repo_tree_reload(cx);
        return;
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
