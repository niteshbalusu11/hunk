impl DiffViewer {
    pub(super) fn set_sidebar_tree_mode(&mut self, mode: SidebarTreeMode, cx: &mut Context<Self>) {
        if self.sidebar_tree_mode == mode {
            if mode == SidebarTreeMode::Files
                && self.repo_tree_nodes.is_empty()
                && !self.repo_tree_loading
            {
                self.request_repo_tree_reload(cx);
            }
            return;
        }

        self.sidebar_tree_mode = mode;
        if mode == SidebarTreeMode::Diff {
            self.right_pane_mode = RightPaneMode::Diff;
            cx.notify();
            return;
        }

        if self.repo_tree_nodes.is_empty() && !self.repo_tree_loading {
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
        cx.notify();
    }

    pub(super) fn select_repo_tree_file(&mut self, path: String, cx: &mut Context<Self>) {
        self.selected_path = Some(path.clone());
        self.selected_status = self
            .files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.status);
        self.right_pane_mode = RightPaneMode::FilePreview;
        self.request_file_preview_reload(path, cx);
        cx.notify();
    }

    pub(super) fn request_repo_tree_reload(&mut self, cx: &mut Context<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.repo_tree_nodes.clear();
            self.repo_tree_file_count = 0;
            self.repo_tree_folder_count = 0;
            self.repo_tree_expanded_dirs.clear();
            self.repo_tree_loading = false;
            self.repo_tree_error = None;
            cx.notify();
            return;
        };

        let epoch = self.next_repo_tree_epoch();
        self.repo_tree_loading = true;
        self.repo_tree_error = None;

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

    fn request_file_preview_reload(&mut self, path: String, cx: &mut Context<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.file_preview_loading = false;
            self.file_preview_error = Some("No repository is open.".to_string());
            self.file_preview_document = None;
            self.file_preview_list_state.reset(0);
            cx.notify();
            return;
        };

        let epoch = self.next_file_preview_epoch();
        self.file_preview_loading = true;
        self.file_preview_error = None;
        self.file_preview_document = None;
        self.file_preview_path = Some(path.clone());
        cx.notify();

        self.file_preview_task = cx.spawn(async move |this, cx| {
            let target_path = path.clone();
            let result = cx.background_executor().spawn(async move {
                load_file_preview(
                    &repo_root,
                    target_path.as_str(),
                    FILE_PREVIEW_MAX_BYTES,
                    FILE_PREVIEW_MAX_LINES,
                )
            });
            let result = result.await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.file_preview_epoch {
                        return;
                    }

                    this.file_preview_loading = false;
                    match result {
                        Ok(document) => {
                            this.file_preview_document = Some(document.clone());
                            this.file_preview_error = None;
                            this.file_preview_list_state.reset(document.lines.len());
                        }
                        Err(err) => {
                            this.file_preview_document = None;
                            this.file_preview_error = Some(format!("Preview unavailable: {err}"));
                            this.file_preview_list_state.reset(0);
                        }
                    }

                    cx.notify();
                })
                .ok();
            }
        });
    }

    fn next_repo_tree_epoch(&mut self) -> usize {
        self.repo_tree_epoch = self.repo_tree_epoch.saturating_add(1);
        self.repo_tree_epoch
    }

    fn next_file_preview_epoch(&mut self) -> usize {
        self.file_preview_epoch = self.file_preview_epoch.saturating_add(1);
        self.file_preview_epoch
    }
}

fn apply_repo_tree_reload(
    this: &mut DiffViewer,
    result: anyhow::Result<Vec<hunk::git::RepoTreeEntry>>,
    cx: &mut Context<DiffViewer>,
) {
    this.repo_tree_loading = false;
    match result {
        Ok(entries) => {
            this.repo_tree_nodes = build_repo_tree(&entries);
            this.repo_tree_file_count =
                count_repo_tree_kind(&this.repo_tree_nodes, RepoTreeNodeKind::File);
            this.repo_tree_folder_count =
                count_repo_tree_kind(&this.repo_tree_nodes, RepoTreeNodeKind::Directory);
            this.repo_tree_error = None;
            this.repo_tree_expanded_dirs
                .retain(|path| repo_tree_has_directory(&this.repo_tree_nodes, path.as_str()));
            if let Some(path) = this.selected_path.clone()
                && this.sidebar_tree_mode == SidebarTreeMode::Files
                && this.right_pane_mode == RightPaneMode::FilePreview
                && !repo_tree_contains_path(&this.repo_tree_nodes, path.as_str())
            {
                this.right_pane_mode = RightPaneMode::Diff;
                this.file_preview_path = None;
                this.file_preview_document = None;
                this.file_preview_error = None;
                this.file_preview_loading = false;
                this.file_preview_list_state.reset(0);
            }
        }
        Err(err) => {
            this.repo_tree_error = Some(format!("Failed to load repository tree: {err:#}"));
            this.repo_tree_nodes.clear();
            this.repo_tree_file_count = 0;
            this.repo_tree_folder_count = 0;
            this.repo_tree_expanded_dirs.clear();
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

fn count_repo_tree_kind(nodes: &[RepoTreeNode], kind: RepoTreeNodeKind) -> usize {
    nodes
        .iter()
        .map(|node| {
            let self_count = usize::from(node.kind == kind);
            self_count + count_repo_tree_kind(&node.children, kind)
        })
        .sum::<usize>()
}
