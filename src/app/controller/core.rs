impl DiffViewer {
    const AUTO_REFRESH_MAX_INTERVAL_MS: u64 = 60_000;
    const AUTO_REFRESH_BACKOFF_STEPS: u32 = 6;
    const REPO_WATCH_DEBOUNCE: Duration = Duration::from_millis(150);

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
        let diff_show_whitespace = config.show_whitespace;
        let diff_show_eol_markers = config.show_eol_markers;
        let branch_input_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Select or create bookmark")
        });
        let commit_input_state = cx
            .new(|cx| InputState::new(window, cx).multi_line(true).rows(4).placeholder("Commit message"));
        let editor_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("text")
                .line_number(true)
                .soft_wrap(false)
                .placeholder("Select a file from Files tree to edit it.")
        });
        let in_app_menu_bar = (!cfg!(target_os = "macos")).then(|| AppMenuBar::new(window, cx));

        let mut view = Self {
            config_store,
            config,
            settings_draft: None,
            state_store,
            state,
            project_path: last_project_path,
            repo_root: None,
            branch_name: "unknown".to_string(),
            branch_has_upstream: false,
            branch_ahead_count: 0,
            branches: Vec::new(),
            bookmark_revisions: Vec::new(),
            files: Vec::new(),
            branch_picker_open: false,
            branch_input_state,
            commit_input_state,
            commit_excluded_files: BTreeSet::new(),
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
            diff_row_segment_cache: Vec::new(),
            diff_visible_file_header_lookup: Vec::new(),
            diff_visible_hunk_header_lookup: Vec::new(),
            file_row_ranges: Vec::new(),
            file_line_stats: BTreeMap::new(),
            diff_list_state: ListState::new(0, ListAlignment::Top, px(360.0)),
            diff_show_whitespace,
            diff_show_eol_markers,
            diff_left_line_number_width: line_number_column_width(DIFF_LINE_NUMBER_MIN_DIGITS),
            diff_right_line_number_width: line_number_column_width(DIFF_LINE_NUMBER_MIN_DIGITS),
            overall_line_stats: LineStats::default(),
            refresh_epoch: 0,
            auto_refresh_unmodified_streak: 0,
            auto_refresh_task: Task::ready(()),
            repo_watch_task: Task::ready(()),
            snapshot_epoch: 0,
            snapshot_task: Task::ready(()),
            snapshot_loading: false,
            last_snapshot_fingerprint: None,
            open_project_task: Task::ready(()),
            patch_epoch: 0,
            patch_task: Task::ready(()),
            patch_loading: false,
            in_app_menu_bar,
            focus_handle: cx.focus_handle(),
            selection_anchor_row: None,
            selection_head_row: None,
            drag_selecting_rows: false,
            scroll_selected_after_reload: true,
            last_visible_row_start: None,
            last_diff_scroll_offset: None,
            last_scroll_activity_at: Instant::now(),
            segment_prefetch_anchor_row: None,
            segment_prefetch_epoch: 0,
            segment_prefetch_task: Task::ready(()),
            fps: 0.0,
            frame_sample_count: 0,
            frame_sample_started_at: Instant::now(),
            fps_epoch: 0,
            fps_task: Task::ready(()),
            repo_discovery_failed: false,
            error_message: None,
            sidebar_collapsed: false,
            workspace_view_mode: WorkspaceViewMode::JjWorkspace,
            sidebar_repo_list_state: ListState::new(
                0,
                ListAlignment::Top,
                px(SIDEBAR_REPO_LIST_ESTIMATED_ROW_HEIGHT),
            ),
            sidebar_repo_row_count: 0,
            repo_tree_nodes: Vec::new(),
            repo_tree_rows: Vec::new(),
            repo_tree_file_count: 0,
            repo_tree_folder_count: 0,
            repo_tree_expanded_dirs: BTreeSet::new(),
            repo_tree_epoch: 0,
            repo_tree_task: Task::ready(()),
            repo_tree_loading: false,
            repo_tree_error: None,
            repo_tree_last_reload: Instant::now(),
            right_pane_mode: RightPaneMode::Diff,
            editor_input_state,
            editor_path: None,
            editor_loading: false,
            editor_error: None,
            editor_dirty: false,
            editor_last_saved_text: None,
            editor_epoch: 0,
            editor_task: Task::ready(()),
            editor_save_loading: false,
            editor_save_epoch: 0,
            editor_save_task: Task::ready(()),
            editor_markdown_preview_task: Task::ready(()),
            editor_markdown_preview_blocks: Vec::new(),
            editor_markdown_preview_loading: false,
            editor_markdown_preview_revision: 0,
            editor_markdown_preview: false,
        };

        let editor_state = view.editor_input_state.clone();
        cx.observe(&editor_state, |this, _, cx| {
            this.sync_editor_dirty_from_input(cx);
            this.schedule_editor_markdown_preview_parse(cx);
        })
        .detach();

        view.apply_theme_preference(window, cx);
        cx.observe_window_appearance(window, |this, window, cx| {
            this.sync_theme_with_system_if_needed(window, cx);
        })
        .detach();

        view.request_snapshot_refresh(cx);
        view.start_auto_refresh(cx);
        view.start_repo_watch(cx);
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

    pub(super) fn request_snapshot_refresh_internal(&mut self, force: bool, cx: &mut Context<Self>) {
        if self.snapshot_loading && !force {
            return;
        }
        if force {
            self.auto_refresh_unmodified_streak = 0;
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
                            this.auto_refresh_unmodified_streak = 0;
                            this.last_snapshot_fingerprint = Some(fingerprint);
                            this.apply_snapshot(snapshot, cx)
                        }
                        Ok(SnapshotRefreshResult::Unchanged(fingerprint)) => {
                            info!("snapshot refresh skipped in {:?} (no repo changes)", elapsed);
                            this.auto_refresh_unmodified_streak =
                                this.auto_refresh_unmodified_streak.saturating_add(1);
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
                    this.start_repo_watch(cx);
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
            bookmark_revisions,
            files,
            line_stats,
            last_commit_subject,
        } = snapshot;

        info!("loaded repository snapshot from {}", root.display());
        let root_changed = self.repo_root.as_ref() != Some(&root);

        let previous_selected_path = self.selected_path.clone();
        let previous_selected_status = self.selected_status;

        self.project_path = Some(root.clone());
        self.set_last_project_path(Some(root.clone()));
        self.repo_root = Some(root);
        self.branch_name = branch_name;
        self.branch_has_upstream = branch_has_upstream;
        self.branch_ahead_count = branch_ahead_count;
        self.branches = branches;
        self.bookmark_revisions = bookmark_revisions;
        self.files = files;
        self.commit_excluded_files
            .retain(|path| self.files.iter().any(|file| file.path == *path));
        self.overall_line_stats = line_stats;
        self.last_commit_subject = last_commit_subject;
        self.repo_discovery_failed = false;
        self.error_message = None;
        if root_changed {
            self.start_repo_watch(cx);
        }
        if root_changed {
            self.commit_excluded_files.clear();
            self.repo_tree_nodes.clear();
            self.repo_tree_rows.clear();
            self.repo_tree_file_count = 0;
            self.repo_tree_folder_count = 0;
            self.repo_tree_expanded_dirs.clear();
            self.sidebar_repo_row_count = 0;
            self.sidebar_repo_list_state.reset(0);
            self.repo_tree_error = None;
            self.right_pane_mode = RightPaneMode::Diff;
            self.clear_editor_state(cx);
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

        if root_changed {
            self.request_repo_tree_reload(cx);
        }

        // Always reload visible diff rows after any loaded snapshot.
        // Fingerprints include more than file lists/counts, and diff text can change while
        // aggregate line stats and selected path stay the same.
        self.scroll_selected_after_reload = selected_changed || self.diff_rows.is_empty();
        self.request_selected_diff_reload(cx);

        cx.notify();
    }

    fn apply_snapshot_error(&mut self, err: anyhow::Error, cx: &mut Context<Self>) {
        let missing_repository = Self::is_missing_repository_error(&err);

        self.cancel_patch_reload();
        self.last_snapshot_fingerprint = None;
        self.repo_root = None;
        self.branch_name = "unknown".to_string();
        self.branch_has_upstream = false;
        self.branch_ahead_count = 0;
        self.branches.clear();
        self.bookmark_revisions.clear();
        self.files.clear();
        self.last_commit_subject = None;
        self.commit_excluded_files.clear();
        self.selected_path = None;
        self.selected_status = None;
        self.overall_line_stats = LineStats::default();
        self.file_row_ranges.clear();
        self.file_line_stats.clear();
        self.diff_row_metadata.clear();
        self.diff_row_segment_cache.clear();
        self.invalidate_segment_prefetch();
        self.diff_visible_file_header_lookup.clear();
        self.diff_visible_hunk_header_lookup.clear();
        self.selection_anchor_row = None;
        self.selection_head_row = None;
        self.drag_selecting_rows = false;
        self.diff_rows = vec![message_row(
            DiffRowKind::Empty,
            "Use File > Open Project... (Cmd/Ctrl+Shift+O) to load a JJ repository.",
        )];
        self.sync_diff_list_state();
        self.recompute_diff_layout();
        self.repo_discovery_failed = missing_repository;
        self.error_message = if missing_repository {
            None
        } else {
            Some(err.to_string())
        };
        self.repo_tree_nodes.clear();
        self.repo_tree_rows.clear();
        self.repo_tree_file_count = 0;
        self.repo_tree_folder_count = 0;
        self.repo_tree_expanded_dirs.clear();
        self.sidebar_repo_row_count = 0;
        self.sidebar_repo_list_state.reset(0);
        self.repo_tree_loading = false;
        self.repo_tree_error = None;
        self.right_pane_mode = RightPaneMode::Diff;
        self.clear_editor_state(cx);
        cx.notify();
    }

    fn is_missing_repository_error(err: &anyhow::Error) -> bool {
        err.chain().any(|cause| {
            let message = cause.to_string();
            message.contains("failed to discover jj repository")
                || message.contains("there is no jj repo")
                || message.contains("failed to discover git repository")
                || message.contains("could not find repository")
        })
    }

    fn request_selected_diff_reload(&mut self, cx: &mut Context<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.cancel_patch_reload();
            self.diff_rows.clear();
            self.diff_row_metadata.clear();
            self.diff_row_segment_cache.clear();
            self.invalidate_segment_prefetch();
            self.diff_visible_file_header_lookup.clear();
            self.diff_visible_hunk_header_lookup.clear();
            self.selection_anchor_row = None;
            self.selection_head_row = None;
            self.drag_selecting_rows = false;
            self.sync_diff_list_state();
            self.file_row_ranges.clear();
            self.file_line_stats.clear();
            self.recompute_diff_layout();
            return;
        };

        if self.files.is_empty() {
            self.cancel_patch_reload();
            self.diff_rows = vec![message_row(DiffRowKind::Empty, "No changed files.")];
            self.diff_row_metadata.clear();
            self.diff_row_segment_cache.clear();
            self.invalidate_segment_prefetch();
            self.diff_visible_file_header_lookup.clear();
            self.diff_visible_hunk_header_lookup.clear();
            self.selection_anchor_row = None;
            self.selection_head_row = None;
            self.drag_selecting_rows = false;
            self.sync_diff_list_state();
            self.file_row_ranges.clear();
            self.file_line_stats.clear();
            self.recompute_diff_layout();
            return;
        }

        let files = self.files.clone();
        let collapsed_files = self.collapsed_files.clone();
        let previous_file_line_stats = self.file_line_stats.clone();
        let expanded_files = files
            .iter()
            .filter(|file| !collapsed_files.contains(file.path.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        let initial_files =
            Self::select_initial_diff_files(&expanded_files, self.selected_path.as_deref());
        let initial_paths = initial_files
            .iter()
            .map(|file| file.path.clone())
            .collect::<BTreeSet<_>>();
        let remaining_files = expanded_files
            .into_iter()
            .filter(|file| !initial_paths.contains(file.path.as_str()))
            .collect::<Vec<_>>();
        let epoch = self.next_patch_epoch();
        self.invalidate_segment_prefetch();
        self.patch_loading = true;
        if self.diff_rows.is_empty() {
            self.diff_rows = vec![message_row(
                DiffRowKind::Meta,
                format!("Loading diffs for {} files...", files.len()),
            )];
            self.diff_row_metadata.clear();
            self.diff_row_segment_cache.clear();
            self.invalidate_segment_prefetch();
            self.diff_visible_file_header_lookup.clear();
            self.diff_visible_hunk_header_lookup.clear();
            self.file_row_ranges.clear();
            self.file_line_stats.clear();
            self.selection_anchor_row = None;
            self.selection_head_row = None;
            self.drag_selecting_rows = false;
            self.sync_diff_list_state();
            self.recompute_diff_layout();
            cx.notify();
        }

        self.patch_task = cx.spawn(async move |this, cx| {
            if initial_files.is_empty() {
                let stream = build_diff_stream_from_patch_map(
                    &files,
                    &collapsed_files,
                    &previous_file_line_stats,
                    &BTreeMap::new(),
                    &BTreeSet::new(),
                );
                if let Some(this) = this.upgrade() {
                    this.update(cx, |this, cx| {
                        if epoch != this.patch_epoch {
                            return;
                        }
                        this.patch_loading = false;
                        this.apply_loaded_diff_stream(stream);
                        cx.notify();
                    })
                    .ok();
                }
                return;
            }

            let mut loaded_patches = BTreeMap::new();
            let mut loading_paths = remaining_files
                .iter()
                .map(|file| file.path.clone())
                .collect::<BTreeSet<_>>();

            let initial_stage_started_at = Instant::now();
            let initial_stage_result = cx
                .background_executor()
                .spawn({
                    let repo_root = repo_root.clone();
                    let files = files.clone();
                    let collapsed_files = collapsed_files.clone();
                    let previous_file_line_stats = previous_file_line_stats.clone();
                    let initial_files = initial_files.clone();
                    let stage_loaded_patches = loaded_patches;
                    let stage_loading_paths = loading_paths;
                    async move {
                        let mut loaded_patches = stage_loaded_patches;
                        let mut loading_paths = stage_loading_paths;
                        let stage_patches = load_patches_for_files(&repo_root, &initial_files)?;
                        loaded_patches.extend(stage_patches);
                        for file in &initial_files {
                            loading_paths.remove(file.path.as_str());
                        }
                        let stream = build_diff_stream_from_patch_map(
                            &files,
                            &collapsed_files,
                            &previous_file_line_stats,
                            &loaded_patches,
                            &loading_paths,
                        );
                        Ok::<_, anyhow::Error>((loaded_patches, loading_paths, stream))
                    }
                })
                .await;

            let (next_loaded_patches, next_loading_paths, initial_stream) = match initial_stage_result {
                Ok(result) => result,
                Err(err) => {
                    if let Some(this) = this.upgrade() {
                        this.update(cx, |this, cx| {
                            if epoch != this.patch_epoch {
                                return;
                            }

                            this.patch_loading = false;
                            let elapsed = initial_stage_started_at.elapsed();
                            error!("initial diff stage failed after {:?}: {err:#}", elapsed);
                            this.apply_diff_stream_error(err);
                            cx.notify();
                        })
                        .ok();
                    }
                    return;
                }
            };
            loaded_patches = next_loaded_patches;
            loading_paths = next_loading_paths;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.patch_epoch {
                        return;
                    }

                    let elapsed = initial_stage_started_at.elapsed();
                    info!(
                        "initial diff stream loaded in {:?} (rows={}, files={})",
                        elapsed,
                        initial_stream.rows.len(),
                        initial_stream.file_ranges.len()
                    );
                    this.apply_loaded_diff_stream(initial_stream);
                    cx.notify();
                })
                .ok();
            }

            if remaining_files.is_empty() {
                if let Some(this) = this.upgrade() {
                    this.update(cx, |this, cx| {
                        if epoch != this.patch_epoch {
                            return;
                        }
                        this.patch_loading = false;
                        cx.notify();
                    })
                    .ok();
                }
                return;
            }

            let total_batches = remaining_files.len().div_ceil(DIFF_PROGRESSIVE_BATCH_FILES);
            for (batch_ix, batch) in remaining_files
                .chunks(DIFF_PROGRESSIVE_BATCH_FILES)
                .enumerate()
            {
                let stage_started_at = Instant::now();
                let stage_files = batch.to_vec();
                let stage_result = cx
                    .background_executor()
                    .spawn({
                        let repo_root = repo_root.clone();
                        let files = files.clone();
                        let collapsed_files = collapsed_files.clone();
                        let previous_file_line_stats = previous_file_line_stats.clone();
                        let stage_loaded_patches = loaded_patches;
                        let stage_loading_paths = loading_paths;
                        async move {
                            let mut loaded_patches = stage_loaded_patches;
                            let mut loading_paths = stage_loading_paths;
                            let stage_patches = load_patches_for_files(&repo_root, &stage_files)?;
                            loaded_patches.extend(stage_patches);
                            for file in &stage_files {
                                loading_paths.remove(file.path.as_str());
                            }
                            let stream = build_diff_stream_from_patch_map(
                                &files,
                                &collapsed_files,
                                &previous_file_line_stats,
                                &loaded_patches,
                                &loading_paths,
                            );
                            Ok::<_, anyhow::Error>((loaded_patches, loading_paths, stream))
                        }
                    })
                    .await;

                let (next_loaded_patches, next_loading_paths, stream) = match stage_result {
                    Ok(result) => result,
                    Err(err) => {
                        if let Some(this) = this.upgrade() {
                            this.update(cx, |this, cx| {
                                if epoch != this.patch_epoch {
                                    return;
                                }

                                this.patch_loading = false;
                                let elapsed = stage_started_at.elapsed();
                                error!(
                                    "progressive diff batch {}/{} failed after {:?}: {err:#}",
                                    batch_ix.saturating_add(1),
                                    total_batches,
                                    elapsed
                                );
                                this.apply_diff_stream_error(err);
                                cx.notify();
                            })
                            .ok();
                        }
                        return;
                    }
                };
                loaded_patches = next_loaded_patches;
                loading_paths = next_loading_paths;

                if let Some(this) = this.upgrade() {
                    this.update(cx, |this, cx| {
                        if epoch != this.patch_epoch {
                            return;
                        }

                        let elapsed = stage_started_at.elapsed();
                        info!(
                            "progressive diff batch {}/{} loaded in {:?} (rows={}, pending_files={})",
                            batch_ix.saturating_add(1),
                            total_batches,
                            elapsed,
                            stream.rows.len(),
                            loading_paths.len()
                        );
                        this.apply_loaded_diff_stream(stream);
                        if batch_ix.saturating_add(1) == total_batches {
                            this.patch_loading = false;
                        }
                        cx.notify();
                    })
                    .ok();
                }
            }
        });
    }

    fn select_initial_diff_files(
        files: &[ChangedFile],
        selected_path: Option<&str>,
    ) -> Vec<ChangedFile> {
        if files.is_empty() {
            return Vec::new();
        }

        if let Some(selected_path) = selected_path
            && let Some(file) = files.iter().find(|file| file.path == selected_path)
        {
            return vec![file.clone()];
        }

        vec![files[0].clone()]
    }

    fn apply_loaded_diff_stream(&mut self, stream: DiffStream) {
        self.invalidate_segment_prefetch();
        self.diff_rows = stream.rows;
        self.diff_row_metadata = stream.row_metadata;
        self.diff_row_segment_cache = stream.row_segments;
        self.clamp_selection_to_rows();
        self.drag_selecting_rows = false;
        self.sync_diff_list_state();
        self.file_row_ranges = stream.file_ranges;
        self.file_line_stats = stream.file_line_stats;
        self.recompute_diff_layout();

        let has_selection = self.selected_path.as_ref().is_some_and(|path| {
            self.files.iter().any(|file| file.path == *path)
        });
        if !has_selection {
            self.selected_path = self.files.first().map(|file| file.path.clone());
        }

        self.selected_status = self.selected_path.as_ref().and_then(|selected| {
            self.files
                .iter()
                .find(|file| &file.path == selected)
                .map(|file| file.status)
        });
        self.last_visible_row_start = None;
        self.recompute_diff_visible_header_lookup();

        if self.scroll_selected_after_reload {
            self.scroll_selected_file_to_top();
            if !self.patch_loading {
                self.scroll_selected_after_reload = false;
            }
        }
    }

    fn apply_diff_stream_error(&mut self, err: anyhow::Error) {
        self.diff_rows = vec![message_row(
            DiffRowKind::Meta,
            format!("Failed to load diff stream: {err:#}"),
        )];
        self.diff_row_metadata.clear();
        self.diff_row_segment_cache.clear();
        self.invalidate_segment_prefetch();
        self.selection_anchor_row = None;
        self.selection_head_row = None;
        self.drag_selecting_rows = false;
        self.sync_diff_list_state();
        self.file_row_ranges.clear();
        self.file_line_stats.clear();
        self.recompute_diff_layout();
        self.diff_visible_file_header_lookup.clear();
        self.diff_visible_hunk_header_lookup.clear();
        self.scroll_selected_after_reload = false;
    }

    fn recompute_diff_visible_header_lookup(&mut self) {
        let row_count = self.diff_rows.len();
        self.diff_visible_file_header_lookup = vec![None; row_count];
        self.diff_visible_hunk_header_lookup = vec![None; row_count];
        if row_count == 0 {
            return;
        }

        if self.diff_row_metadata.len() == row_count {
            let mut current_file_header = None::<usize>;
            let mut current_hunk_header = None::<usize>;
            for row_ix in 0..row_count {
                let meta = &self.diff_row_metadata[row_ix];
                match meta.kind {
                    DiffStreamRowKind::EmptyState => {
                        current_file_header = None;
                        current_hunk_header = None;
                    }
                    DiffStreamRowKind::FileHeader => {
                        current_file_header = Some(row_ix);
                        current_hunk_header = None;
                    }
                    DiffStreamRowKind::CoreHunkHeader => {
                        if current_file_header.is_none() {
                            current_file_header = self.file_row_ranges.iter().find_map(|range| {
                                if row_ix >= range.start_row && row_ix < range.end_row {
                                    Some(range.start_row)
                                } else {
                                    None
                                }
                            });
                        }
                        current_hunk_header = Some(row_ix);
                    }
                    _ => {}
                }

                self.diff_visible_file_header_lookup[row_ix] = current_file_header;
                self.diff_visible_hunk_header_lookup[row_ix] = current_hunk_header;
            }
            return;
        }

        let mut current_hunk_header = None::<usize>;
        for row_ix in 0..row_count {
            if self
                .diff_rows
                .get(row_ix)
                .is_some_and(|row| row.kind == DiffRowKind::HunkHeader)
            {
                current_hunk_header = Some(row_ix);
            }

            let file_header_ix = self.file_row_ranges.iter().find_map(|range| {
                if row_ix >= range.start_row && row_ix < range.end_row {
                    Some(range.start_row)
                } else {
                    None
                }
            });
            self.diff_visible_file_header_lookup[row_ix] = file_header_ix;
            self.diff_visible_hunk_header_lookup[row_ix] = current_hunk_header;
        }
    }

    fn next_snapshot_epoch(&mut self) -> usize {
        self.snapshot_epoch = self.snapshot_epoch.saturating_add(1);
        self.snapshot_epoch
    }

    fn auto_refresh_interval(&self) -> Duration {
        if self.config.auto_refresh_interval_ms == 0 {
            return Duration::ZERO;
        }

        let base_ms = self.config.auto_refresh_interval_ms.max(250);
        let backoff_factor =
            1_u64 << self.auto_refresh_unmodified_streak.min(Self::AUTO_REFRESH_BACKOFF_STEPS);
        let interval_ms = (base_ms.saturating_mul(backoff_factor))
            .min(Self::AUTO_REFRESH_MAX_INTERVAL_MS);
        Duration::from_millis(interval_ms)
    }

    fn should_ignore_repo_watch_path(
        path: &std::path::Path,
        repo_root: &std::path::Path,
    ) -> bool {
        let Ok(relative_path) = path.strip_prefix(repo_root) else {
            return false;
        };

        relative_path
            .components()
            .any(|component| component.as_os_str() == ".jj" || component.as_os_str() == ".git")
    }

    fn start_repo_watch(&mut self, cx: &mut Context<Self>) {
        self.repo_watch_task = Task::ready(());

        let Some(repo_root) = self.repo_root.clone().or_else(|| self.project_path.clone()) else {
            return;
        };
        let (event_tx, mut event_rx) = mpsc::unbounded::<notify::Result<notify::Event>>();
        let repo_root_path = repo_root.clone();
        let repo_root_for_cb = repo_root.to_string_lossy().to_string();
        let watcher = notify::recommended_watcher(move |result| {
            event_tx.unbounded_send(result).ok();
        });

        let mut watcher = match watcher {
            Ok(watcher) => watcher,
            Err(err) => {
                error!("failed to start file watch for {}: {err}", repo_root_for_cb);
                return;
            }
        };

        if let Err(err) = watcher.watch(&repo_root, notify::RecursiveMode::Recursive) {
            error!("failed to watch repository at {}: {err}", repo_root_for_cb);
            return;
        }

        self.repo_watch_task = cx.spawn(async move |this, cx| {
            let mut last_event_at = Instant::now() - Self::REPO_WATCH_DEBOUNCE;
            while let Some(event) = event_rx.next().await {
                let Ok(event) = event else {
                    continue;
                };

                if event
                    .paths
                    .iter()
                    .all(|path| Self::should_ignore_repo_watch_path(path, &repo_root_path))
                {
                    continue;
                }

                let now = Instant::now();
                if now.duration_since(last_event_at) < Self::REPO_WATCH_DEBOUNCE {
                    continue;
                }
                last_event_at = now;

                if let Some(this) = this.upgrade() {
                    this.update(cx, |this, cx| {
                        this.request_snapshot_refresh_internal(true, cx);
                        if this.workspace_view_mode == WorkspaceViewMode::Files {
                            this.request_repo_tree_reload(cx);
                        }
                    })
                    .ok();
                }
            }
            drop(watcher);
        });
    }

    fn next_patch_epoch(&mut self) -> usize {
        self.patch_epoch = self.patch_epoch.saturating_add(1);
        self.patch_epoch
    }

    fn cancel_patch_reload(&mut self) {
        self.next_patch_epoch();
        self.patch_task = Task::ready(());
        self.patch_loading = false;
    }

    fn next_segment_prefetch_epoch(&mut self) -> usize {
        self.segment_prefetch_epoch = self.segment_prefetch_epoch.saturating_add(1);
        self.segment_prefetch_epoch
    }

    fn invalidate_segment_prefetch(&mut self) {
        self.next_segment_prefetch_epoch();
        self.segment_prefetch_task = Task::ready(());
        self.segment_prefetch_anchor_row = None;
    }

    fn start_auto_refresh(&mut self, cx: &mut Context<Self>) {
        let epoch = self.next_refresh_epoch();
        if self.config.auto_refresh_interval_ms == 0 {
            return;
        }

        let interval = self.auto_refresh_interval();
        self.schedule_auto_refresh(epoch, interval, cx);
    }

    pub(super) fn restart_auto_refresh(&mut self, cx: &mut Context<Self>) {
        self.auto_refresh_task = Task::ready(());
        self.auto_refresh_unmodified_streak = 0;
        if self.config.auto_refresh_interval_ms == 0 {
            return;
        }

        let epoch = self.next_refresh_epoch();
        let interval = self.auto_refresh_interval();
        self.schedule_auto_refresh(epoch, interval, cx);
    }

    fn next_refresh_epoch(&mut self) -> usize {
        self.refresh_epoch = self.refresh_epoch.saturating_add(1);
        self.refresh_epoch
    }

    fn schedule_auto_refresh(
        &mut self,
        epoch: usize,
        delay: Duration,
        cx: &mut Context<Self>,
    ) {
        if epoch != self.refresh_epoch {
            return;
        }
        if delay == Duration::ZERO || self.config.auto_refresh_interval_ms == 0 {
            return;
        }

        self.auto_refresh_task = cx.spawn(async move |this, cx| {
            Timer::after(delay).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if this.config.auto_refresh_interval_ms == 0 {
                        return;
                    }

                    if this.recently_scrolling() {
                        let next_epoch = this.next_refresh_epoch();
                        let next_delay = this.auto_refresh_interval();
                        this.schedule_auto_refresh(next_epoch, next_delay, cx);
                        return;
                    }

                    if this.project_path.is_some() {
                        this.request_snapshot_refresh(cx);
                    }

                    let next_delay = this.auto_refresh_interval();
                    let next_epoch = this.next_refresh_epoch();
                    this.schedule_auto_refresh(next_epoch, next_delay, cx);
                })
                .ok();
            }
        });
    }

    fn recently_scrolling(&self) -> bool {
        self.last_scroll_activity_at.elapsed() < AUTO_REFRESH_SCROLL_DEBOUNCE
    }
}
