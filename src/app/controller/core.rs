impl DiffViewer {
    fn load_app_config() -> (Option<ConfigStore>, AppConfig) {
        let store = match ConfigStore::new() {
            Ok(store) => store,
            Err(err) => {
                error!("failed to initialize config path: {err:#}");
                return (None, AppConfig::default());
            }
        };

        match store.load_or_create_default() {
            Ok(config) => (Some(store), config),
            Err(err) => {
                error!(
                    "failed to load app config from {}: {err:#}",
                    store.path().display()
                );
                (Some(store), AppConfig::default())
            }
        }
    }

    fn load_app_state() -> (Option<AppStateStore>, AppState) {
        let store = match AppStateStore::new() {
            Ok(store) => store,
            Err(err) => {
                error!("failed to initialize app state path: {err:#}");
                return (None, AppState::default());
            }
        };

        match store.load_or_default() {
            Ok(state) => (Some(store), state),
            Err(err) => {
                error!("failed to load app state from {}: {err:#}", store.path().display());
                (Some(store), AppState::default())
            }
        }
    }

    fn load_legacy_last_project_path(config_store: &ConfigStore) -> Option<PathBuf> {
        let raw = std::fs::read_to_string(config_store.path()).ok()?;
        let value = raw.parse::<toml::Value>().ok()?;
        value
            .get("last_project_path")
            .and_then(toml::Value::as_str)
            .map(PathBuf::from)
    }

    fn apply_theme_preference(&self, window: &mut Window, cx: &mut Context<Self>) {
        let mode = match self.config.theme {
            ThemePreference::System => ThemeMode::from(window.appearance()),
            ThemePreference::Light => ThemeMode::Light,
            ThemePreference::Dark => ThemeMode::Dark,
        };
        Theme::change(mode, Some(window), cx);
    }

    fn persist_config(&self) {
        let Some(store) = &self.config_store else {
            return;
        };

        if let Err(err) = store.save(&self.config) {
            error!(
                "failed to save app config to {}: {err:#}",
                store.path().display()
            );
        }
    }

    fn persist_state(&self) {
        let Some(store) = &self.state_store else {
            return;
        };

        if let Err(err) = store.save(&self.state) {
            error!(
                "failed to save app state to {}: {err:#}",
                store.path().display()
            );
        }
    }

    fn set_last_project_path(&mut self, project_path: Option<PathBuf>) {
        if self.state.last_project_path == project_path {
            return;
        }

        self.state.last_project_path = project_path;
        self.persist_state();
    }

    fn sync_theme_with_system_if_needed(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.config.theme != ThemePreference::System {
            return;
        }
        self.apply_theme_preference(window, cx);
    }

    pub(super) fn set_theme_preference(
        &mut self,
        theme: ThemePreference,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.config.theme == theme {
            return;
        }

        self.config.theme = theme;
        self.apply_theme_preference(window, cx);
        self.persist_config();
        cx.notify();
    }

    pub(super) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (config_store, config) = Self::load_app_config();
        let (state_store, mut state) = Self::load_app_state();
        if state.last_project_path.is_none()
            && let Some(config_store) = config_store.as_ref()
            && let Some(last_project_path) = Self::load_legacy_last_project_path(config_store)
        {
            state.last_project_path = Some(last_project_path);
            if let Some(state_store) = state_store.as_ref()
                && let Err(err) = state_store.save(&state)
            {
                error!(
                    "failed to migrate app state to {}: {err:#}",
                    state_store.path().display()
                );
            }
        }
        let last_project_path = state.last_project_path.clone();
        let diff_fit_to_width = matches!(config.diff_view, DiffViewMode::Fit);
        let diff_show_whitespace = config.show_whitespace;
        let diff_show_eol_markers = config.show_eol_markers;
        let tree_state = cx.new(|cx| TreeState::new(cx));
        let branch_input_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Select or create branch")
        });
        let commit_input_state = cx
            .new(|cx| InputState::new(window, cx).multi_line(true).rows(4).placeholder("Commit message"));

        let mut view = Self {
            config_store,
            config,
            state_store,
            state,
            project_path: last_project_path,
            repo_root: None,
            branch_name: "unknown".to_string(),
            branch_has_upstream: false,
            branch_ahead_count: 0,
            branches: Vec::new(),
            files: Vec::new(),
            branch_picker_open: false,
            branch_input_state,
            commit_input_state,
            last_commit_subject: None,
            git_action_epoch: 0,
            git_action_task: Task::ready(()),
            git_action_loading: false,
            git_status_message: None,
            collapsed_files: BTreeSet::new(),
            selected_path: None,
            selected_status: None,
            diff_rows: Vec::new(),
            diff_row_metadata: Vec::new(),
            diff_row_segment_cache: BTreeMap::new(),
            file_row_ranges: Vec::new(),
            file_line_stats: BTreeMap::new(),
            diff_list_state: ListState::new(0, ListAlignment::Top, px(360.0)),
            diff_horizontal_scroll_handle: ScrollHandle::new(),
            diff_fit_to_width,
            diff_show_whitespace,
            diff_show_eol_markers,
            diff_left_column_width: DIFF_MIN_COLUMN_WIDTH,
            diff_right_column_width: DIFF_MIN_COLUMN_WIDTH,
            diff_pan_content_width: DIFF_MIN_CONTENT_WIDTH,
            diff_left_line_number_width: line_number_column_width(DIFF_LINE_NUMBER_MIN_DIGITS),
            diff_right_line_number_width: line_number_column_width(DIFF_LINE_NUMBER_MIN_DIGITS),
            overall_line_stats: LineStats::default(),
            refresh_epoch: 0,
            auto_refresh_task: Task::ready(()),
            snapshot_epoch: 0,
            snapshot_task: Task::ready(()),
            snapshot_loading: false,
            last_snapshot_fingerprint: None,
            open_project_task: Task::ready(()),
            patch_epoch: 0,
            patch_task: Task::ready(()),
            patch_loading: false,
            focus_handle: cx.focus_handle(),
            selection_anchor_row: None,
            selection_head_row: None,
            drag_selecting_rows: false,
            scroll_selected_after_reload: true,
            last_visible_row_start: None,
            last_diff_scroll_offset: None,
            last_scroll_activity_at: Instant::now(),
            fps: 0.0,
            frame_sample_count: 0,
            frame_sample_started_at: Instant::now(),
            fps_epoch: 0,
            fps_task: Task::ready(()),
            repo_discovery_failed: false,
            error_message: None,
            tree_state,
            sidebar_tree_mode: SidebarTreeMode::Diff,
            repo_tree_nodes: Vec::new(),
            repo_tree_file_count: 0,
            repo_tree_folder_count: 0,
            repo_tree_expanded_dirs: BTreeSet::new(),
            repo_tree_epoch: 0,
            repo_tree_task: Task::ready(()),
            repo_tree_loading: false,
            repo_tree_error: None,
            right_pane_mode: RightPaneMode::Diff,
            file_preview_path: None,
            file_preview_document: None,
            file_preview_loading: false,
            file_preview_error: None,
            file_preview_epoch: 0,
            file_preview_task: Task::ready(()),
            file_preview_list_state: ListState::new(0, ListAlignment::Top, px(22.0)),
        };

        view.apply_theme_preference(window, cx);
        cx.observe_window_appearance(window, |this, window, cx| {
            this.sync_theme_with_system_if_needed(window, cx);
        })
        .detach();

        view.request_snapshot_refresh(cx);
        view.start_auto_refresh(cx);
        view.start_fps_monitor(cx);
        view
    }

    pub(super) fn open_project_action(
        &mut self,
        _: &OpenProject,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_project_picker(cx);
    }

    pub(super) fn select_file(&mut self, path: String, cx: &mut Context<Self>) {
        self.selected_path = Some(path.clone());
        self.selected_status = self
            .files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.status);
        self.right_pane_mode = RightPaneMode::Diff;
        self.scroll_to_file_start(&path);
        self.last_visible_row_start = None;
        self.last_diff_scroll_offset = None;
        self.last_scroll_activity_at = Instant::now();
        cx.notify();
    }

    pub(super) fn request_snapshot_refresh(&mut self, cx: &mut Context<Self>) {
        self.request_snapshot_refresh_internal(false, cx);
    }

    fn request_snapshot_refresh_internal(&mut self, force: bool, cx: &mut Context<Self>) {
        if self.snapshot_loading && !force {
            return;
        }

        enum SnapshotRefreshResult {
            Unchanged(RepoSnapshotFingerprint),
            Loaded {
                fingerprint: RepoSnapshotFingerprint,
                snapshot: RepoSnapshot,
            },
        }

        let source_dir_result = self
            .project_path
            .clone()
            .map(Ok)
            .unwrap_or_else(|| std::env::current_dir().context("failed to resolve current directory"));
        let previous_fingerprint = if force {
            None
        } else {
            self.last_snapshot_fingerprint.clone()
        };
        let epoch = self.next_snapshot_epoch();
        self.snapshot_loading = true;

        self.snapshot_task = cx.spawn(async move |this, cx| {
            let started_at = Instant::now();
            let result = match source_dir_result {
                Ok(source_dir) => {
                    cx.background_executor()
                        .spawn(async move {
                            let fingerprint = load_snapshot_fingerprint(&source_dir)?;
                            if previous_fingerprint.as_ref() == Some(&fingerprint) {
                                return Ok(SnapshotRefreshResult::Unchanged(fingerprint));
                            }

                            let snapshot = load_snapshot(&source_dir)?;
                            Ok(SnapshotRefreshResult::Loaded {
                                fingerprint,
                                snapshot,
                            })
                        })
                        .await
                }
                Err(err) => Err(err),
            };

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.snapshot_epoch {
                        return;
                    }

                    this.snapshot_loading = false;
                    let elapsed = started_at.elapsed();
                    match result {
                        Ok(SnapshotRefreshResult::Loaded {
                            fingerprint,
                            snapshot,
                        }) => {
                            info!("snapshot refresh completed in {:?}", elapsed);
                            this.last_snapshot_fingerprint = Some(fingerprint);
                            this.apply_snapshot(snapshot, cx)
                        }
                        Ok(SnapshotRefreshResult::Unchanged(fingerprint)) => {
                            info!("snapshot refresh skipped in {:?} (no repo changes)", elapsed);
                            this.last_snapshot_fingerprint = Some(fingerprint);
                            cx.notify();
                        }
                        Err(err) => {
                            error!("snapshot refresh failed after {:?}: {err:#}", elapsed);
                            this.apply_snapshot_error(err, cx)
                        }
                    }
                })
                .ok();
            }
        });
    }

    pub(super) fn open_project_picker(&mut self, cx: &mut Context<Self>) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open Project".into()),
        });

        self.open_project_task = cx.spawn(async move |this, cx| {
            let selection = match prompt.await {
                Ok(selection) => selection,
                Err(err) => {
                    error!("project picker prompt channel closed: {err}");
                    return;
                }
            };

            let selected_path = match selection {
                Ok(Some(paths)) => paths.into_iter().next(),
                Ok(None) => None,
                Err(err) => {
                    if let Some(this) = this.upgrade() {
                        this.update(cx, |this, cx| {
                            this.git_status_message =
                                Some(format!("Failed to open folder picker: {err:#}"));
                            cx.notify();
                        })
                        .ok();
                    }
                    return;
                }
            };

            let Some(selected_path) = selected_path else {
                return;
            };

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    this.project_path = Some(selected_path.clone());
                    this.set_last_project_path(Some(selected_path));
                    this.git_status_message = None;
                    this.request_snapshot_refresh_internal(true, cx);
                    cx.notify();
                })
                .ok();
            }
        });
    }

    fn apply_snapshot(&mut self, snapshot: RepoSnapshot, cx: &mut Context<Self>) {
        let RepoSnapshot {
            root,
            branch_name,
            branch_has_upstream,
            branch_ahead_count,
            branches,
            files,
            line_stats,
            last_commit_subject,
        } = snapshot;

        info!("loaded repository snapshot from {}", root.display());
        let root_changed = self.repo_root.as_ref() != Some(&root);

        let files_changed = self.files != files;
        let overall_changed = self.overall_line_stats != line_stats;
        let previous_selected_path = self.selected_path.clone();
        let previous_selected_status = self.selected_status;

        self.project_path = Some(root.clone());
        self.set_last_project_path(Some(root.clone()));
        self.repo_root = Some(root);
        self.branch_name = branch_name;
        self.branch_has_upstream = branch_has_upstream;
        self.branch_ahead_count = branch_ahead_count;
        self.branches = branches;
        self.files = files;
        self.overall_line_stats = line_stats;
        self.last_commit_subject = last_commit_subject;
        self.repo_discovery_failed = false;
        self.error_message = None;
        if root_changed {
            self.repo_tree_nodes.clear();
            self.repo_tree_file_count = 0;
            self.repo_tree_folder_count = 0;
            self.repo_tree_expanded_dirs.clear();
            self.repo_tree_error = None;
            self.right_pane_mode = RightPaneMode::Diff;
            self.file_preview_path = None;
            self.file_preview_document = None;
            self.file_preview_loading = false;
            self.file_preview_error = None;
            self.file_preview_list_state.reset(0);
        }
        self.collapsed_files
            .retain(|path| self.files.iter().any(|file| file.path == *path));

        let current_selection = self
            .selected_path
            .as_ref()
            .filter(|selected| self.files.iter().any(|file| &file.path == *selected))
            .cloned();
        self.selected_path =
            current_selection.or_else(|| self.files.first().map(|file| file.path.clone()));
        self.selected_status = self.selected_path.as_ref().and_then(|selected| {
            self.files
                .iter()
                .find(|file| &file.path == selected)
                .map(|file| file.status)
        });

        let selected_changed = self.selected_path != previous_selected_path
            || self.selected_status != previous_selected_status;

        if files_changed {
            self.rebuild_tree(cx);
        }
        if root_changed {
            self.request_repo_tree_reload(cx);
        }

        if files_changed || overall_changed || selected_changed || self.diff_rows.is_empty() {
            self.scroll_selected_after_reload = selected_changed || self.diff_rows.is_empty();
            self.request_selected_diff_reload(cx);
        }

        cx.notify();
    }

    fn apply_snapshot_error(&mut self, err: anyhow::Error, cx: &mut Context<Self>) {
        let missing_repository = Self::is_missing_repository_error(&err);

        self.last_snapshot_fingerprint = None;
        self.repo_root = None;
        self.branch_name = "unknown".to_string();
        self.branch_has_upstream = false;
        self.branch_ahead_count = 0;
        self.branches.clear();
        self.files.clear();
        self.last_commit_subject = None;
        self.selected_path = None;
        self.selected_status = None;
        self.overall_line_stats = LineStats::default();
        self.file_row_ranges.clear();
        self.file_line_stats.clear();
        self.diff_row_metadata.clear();
        self.diff_row_segment_cache.clear();
        self.selection_anchor_row = None;
        self.selection_head_row = None;
        self.drag_selecting_rows = false;
        self.diff_rows = vec![message_row(
            DiffRowKind::Empty,
            "Use File > Open Project... (Cmd/Ctrl+Shift+O) to load a Git repository.",
        )];
        self.sync_diff_list_state();
        self.recompute_diff_pan_layout();
        self.repo_discovery_failed = missing_repository;
        self.error_message = if missing_repository {
            None
        } else {
            Some(err.to_string())
        };
        self.repo_tree_nodes.clear();
        self.repo_tree_file_count = 0;
        self.repo_tree_folder_count = 0;
        self.repo_tree_expanded_dirs.clear();
        self.repo_tree_loading = false;
        self.repo_tree_error = None;
        self.right_pane_mode = RightPaneMode::Diff;
        self.file_preview_path = None;
        self.file_preview_document = None;
        self.file_preview_loading = false;
        self.file_preview_error = None;
        self.file_preview_list_state.reset(0);
        self.rebuild_tree(cx);
        cx.notify();
    }

    fn is_missing_repository_error(err: &anyhow::Error) -> bool {
        err.chain().any(|cause| {
            let message = cause.to_string();
            message.contains("failed to discover git repository")
                || message.contains("could not find repository")
        })
    }

    fn request_selected_diff_reload(&mut self, cx: &mut Context<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.diff_rows.clear();
            self.diff_row_metadata.clear();
            self.diff_row_segment_cache.clear();
            self.selection_anchor_row = None;
            self.selection_head_row = None;
            self.drag_selecting_rows = false;
            self.sync_diff_list_state();
            self.file_row_ranges.clear();
            self.file_line_stats.clear();
            self.recompute_diff_pan_layout();
            self.patch_loading = false;
            return;
        };

        if self.files.is_empty() {
            self.diff_rows = vec![message_row(DiffRowKind::Empty, "No changed files.")];
            self.diff_row_metadata.clear();
            self.diff_row_segment_cache.clear();
            self.selection_anchor_row = None;
            self.selection_head_row = None;
            self.drag_selecting_rows = false;
            self.sync_diff_list_state();
            self.file_row_ranges.clear();
            self.file_line_stats.clear();
            self.recompute_diff_pan_layout();
            self.patch_loading = false;
            return;
        }

        let files = self.files.clone();
        let collapsed_files = self.collapsed_files.clone();
        let previous_file_line_stats = self.file_line_stats.clone();
        let epoch = self.next_patch_epoch();
        self.patch_loading = true;

        self.patch_task = cx.spawn(async move |this, cx| {
            let started_at = Instant::now();
            let result = cx
                .background_executor()
                .spawn(async move {
                    load_diff_stream(
                        &repo_root,
                        &files,
                        &collapsed_files,
                        &previous_file_line_stats,
                    )
                })
                .await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.patch_epoch {
                        return;
                    }

                    this.patch_loading = false;
                    let elapsed = started_at.elapsed();
                    match result {
                        Ok(stream) => {
                            info!(
                                "diff stream loaded in {:?} (rows={}, files={})",
                                elapsed,
                                stream.rows.len(),
                                stream.file_ranges.len()
                            );
                            this.diff_rows = stream.rows;
                            this.diff_row_metadata = stream.row_metadata;
                            this.diff_row_segment_cache = stream.row_segments;
                            this.clamp_selection_to_rows();
                            this.drag_selecting_rows = false;
                            this.sync_diff_list_state();
                            this.file_row_ranges = stream.file_ranges;
                            this.file_line_stats = stream.file_line_stats;
                            this.recompute_diff_pan_layout();

                            let has_selection = this.selected_path.as_ref().is_some_and(|path| {
                                this.files.iter().any(|file| file.path == *path)
                            });
                            if !has_selection {
                                this.selected_path =
                                    this.files.first().map(|file| file.path.clone());
                            }

                            this.selected_status =
                                this.selected_path.as_ref().and_then(|selected| {
                                    this.files
                                        .iter()
                                        .find(|file| &file.path == selected)
                                        .map(|file| file.status)
                                });
                            this.last_visible_row_start = None;

                            if this.scroll_selected_after_reload {
                                this.scroll_selected_after_reload = false;
                                this.scroll_selected_file_to_top();
                            }
                        }
                        Err(err) => {
                            error!("diff stream load failed after {:?}: {err:#}", elapsed);
                            this.diff_rows = vec![message_row(
                                DiffRowKind::Meta,
                                format!("Failed to load diff stream: {err:#}"),
                            )];
                            this.diff_row_metadata.clear();
                            this.diff_row_segment_cache.clear();
                            this.selection_anchor_row = None;
                            this.selection_head_row = None;
                            this.drag_selecting_rows = false;
                            this.sync_diff_list_state();
                            this.file_row_ranges.clear();
                            this.file_line_stats.clear();
                            this.recompute_diff_pan_layout();
                            this.scroll_selected_after_reload = false;
                        }
                    }

                    cx.notify();
                })
                .ok();
            }
        });
    }

    fn next_snapshot_epoch(&mut self) -> usize {
        self.snapshot_epoch = self.snapshot_epoch.saturating_add(1);
        self.snapshot_epoch
    }

    fn next_patch_epoch(&mut self) -> usize {
        self.patch_epoch = self.patch_epoch.saturating_add(1);
        self.patch_epoch
    }

    fn rebuild_tree(&mut self, cx: &mut Context<Self>) {
        let items = build_tree_items(&self.files);
        self.tree_state
            .update(cx, |state, cx| state.set_items(items, cx));
    }

    fn start_auto_refresh(&mut self, cx: &mut Context<Self>) {
        let epoch = self.next_refresh_epoch();
        self.schedule_auto_refresh(epoch, cx);
    }

    fn next_refresh_epoch(&mut self) -> usize {
        self.refresh_epoch = self.refresh_epoch.saturating_add(1);
        self.refresh_epoch
    }

    fn schedule_auto_refresh(&mut self, epoch: usize, cx: &mut Context<Self>) {
        if epoch != self.refresh_epoch {
            return;
        }

        self.auto_refresh_task = cx.spawn(async move |this, cx| {
            Timer::after(AUTO_REFRESH_INTERVAL).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if this.recently_scrolling() {
                        let next_epoch = this.next_refresh_epoch();
                        this.schedule_auto_refresh(next_epoch, cx);
                        return;
                    }

                    this.request_snapshot_refresh(cx);
                    let next_epoch = this.next_refresh_epoch();
                    this.schedule_auto_refresh(next_epoch, cx);
                })
                .ok();
            }
        });
    }

    fn recently_scrolling(&self) -> bool {
        self.last_scroll_activity_at.elapsed() < AUTO_REFRESH_SCROLL_DEBOUNCE
    }
}
