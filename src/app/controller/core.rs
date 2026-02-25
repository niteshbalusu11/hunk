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
        let diff_fit_to_width = matches!(config.diff_view, DiffViewMode::Fit);
        let tree_state = cx.new(|cx| TreeState::new(cx));

        let mut view = Self {
            config_store,
            config,
            repo_root: None,
            branch_name: "unknown".to_string(),
            files: Vec::new(),
            collapsed_files: BTreeSet::new(),
            selected_path: None,
            selected_status: None,
            diff_rows: Vec::new(),
            diff_row_metadata: Vec::new(),
            file_row_ranges: Vec::new(),
            file_line_stats: BTreeMap::new(),
            diff_list_state: ListState::new(0, ListAlignment::Top, px(360.0)),
            diff_horizontal_scroll_handle: ScrollHandle::new(),
            diff_fit_to_width,
            diff_left_column_width: DIFF_MIN_COLUMN_WIDTH,
            diff_right_column_width: DIFF_MIN_COLUMN_WIDTH,
            diff_pan_content_width: DIFF_MIN_CONTENT_WIDTH,
            diff_left_line_number_width: line_number_column_width(DIFF_LINE_NUMBER_MIN_DIGITS),
            diff_right_line_number_width: line_number_column_width(DIFF_LINE_NUMBER_MIN_DIGITS),
            overall_line_stats: LineStats::default(),
            selected_line_stats: LineStats::default(),
            refresh_epoch: 0,
            auto_refresh_task: Task::ready(()),
            snapshot_epoch: 0,
            snapshot_task: Task::ready(()),
            snapshot_loading: false,
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
            error_message: None,
            tree_state,
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

    pub(super) fn select_file(&mut self, path: String, cx: &mut Context<Self>) {
        self.selected_path = Some(path.clone());
        self.selected_status = self
            .files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.status);
        self.sync_selected_line_stats();
        self.scroll_to_file_start(&path);
        self.last_visible_row_start = None;
        self.last_diff_scroll_offset = None;
        self.last_scroll_activity_at = Instant::now();
        cx.notify();
    }

    pub(super) fn request_snapshot_refresh(&mut self, cx: &mut Context<Self>) {
        if self.snapshot_loading {
            return;
        }

        let cwd_result = std::env::current_dir().context("failed to resolve current directory");
        let epoch = self.next_snapshot_epoch();
        self.snapshot_loading = true;

        self.snapshot_task = cx.spawn(async move |this, cx| {
            let result = match cwd_result {
                Ok(cwd) => {
                    cx.background_executor()
                        .spawn(async move { load_snapshot(&cwd) })
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
                    match result {
                        Ok(snapshot) => this.apply_snapshot(snapshot, cx),
                        Err(err) => this.apply_snapshot_error(err, cx),
                    }
                })
                .ok();
            }
        });
    }

    fn apply_snapshot(&mut self, snapshot: RepoSnapshot, cx: &mut Context<Self>) {
        info!(
            "loaded repository snapshot from {}",
            snapshot.root.display()
        );

        let files_changed = self.files != snapshot.files;
        let overall_changed = self.overall_line_stats != snapshot.line_stats;
        let previous_selected_path = self.selected_path.clone();
        let previous_selected_status = self.selected_status;

        self.repo_root = Some(snapshot.root);
        self.branch_name = snapshot.branch_name;
        self.files = snapshot.files;
        self.overall_line_stats = snapshot.line_stats;
        self.error_message = None;
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

        if files_changed || overall_changed || selected_changed || self.diff_rows.is_empty() {
            self.scroll_selected_after_reload = selected_changed || self.diff_rows.is_empty();
            self.request_selected_diff_reload(cx);
        } else {
            self.sync_selected_line_stats();
        }

        cx.notify();
    }

    fn apply_snapshot_error(&mut self, err: anyhow::Error, cx: &mut Context<Self>) {
        self.repo_root = None;
        self.branch_name = "unknown".to_string();
        self.files.clear();
        self.selected_path = None;
        self.selected_status = None;
        self.overall_line_stats = LineStats::default();
        self.selected_line_stats = LineStats::default();
        self.file_row_ranges.clear();
        self.file_line_stats.clear();
        self.diff_row_metadata.clear();
        self.selection_anchor_row = None;
        self.selection_head_row = None;
        self.drag_selecting_rows = false;
        self.diff_rows = vec![message_row(
            DiffRowKind::Empty,
            "Open this app from a Git repository to load diffs.",
        )];
        self.sync_diff_list_state();
        self.recompute_diff_pan_layout();
        self.error_message = Some(err.to_string());
        self.rebuild_tree(cx);
        cx.notify();
    }

    fn request_selected_diff_reload(&mut self, cx: &mut Context<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.diff_rows.clear();
            self.diff_row_metadata.clear();
            self.selection_anchor_row = None;
            self.selection_head_row = None;
            self.drag_selecting_rows = false;
            self.sync_diff_list_state();
            self.file_row_ranges.clear();
            self.file_line_stats.clear();
            self.selected_line_stats = LineStats::default();
            self.recompute_diff_pan_layout();
            self.patch_loading = false;
            return;
        };

        if self.files.is_empty() {
            self.diff_rows = vec![message_row(DiffRowKind::Empty, "No changed files.")];
            self.diff_row_metadata.clear();
            self.selection_anchor_row = None;
            self.selection_head_row = None;
            self.drag_selecting_rows = false;
            self.sync_diff_list_state();
            self.file_row_ranges.clear();
            self.file_line_stats.clear();
            self.selected_line_stats = LineStats::default();
            self.recompute_diff_pan_layout();
            self.patch_loading = false;
            return;
        }

        let files = self.files.clone();
        let collapsed_files = self.collapsed_files.clone();
        let epoch = self.next_patch_epoch();
        self.patch_loading = true;

        self.patch_task = cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { load_diff_stream(&repo_root, &files, &collapsed_files) })
                .await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.patch_epoch {
                        return;
                    }

                    this.patch_loading = false;
                    match result {
                        Ok(stream) => {
                            this.diff_rows = stream.rows;
                            this.diff_row_metadata = stream.row_metadata;
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
                            this.sync_selected_line_stats();
                            this.last_visible_row_start = None;

                            if this.scroll_selected_after_reload {
                                this.scroll_selected_after_reload = false;
                                this.scroll_selected_file_to_top();
                            }
                        }
                        Err(err) => {
                            this.diff_rows = vec![message_row(
                                DiffRowKind::Meta,
                                format!("Failed to load diff stream: {err:#}"),
                            )];
                            this.diff_row_metadata.clear();
                            this.selection_anchor_row = None;
                            this.selection_head_row = None;
                            this.drag_selecting_rows = false;
                            this.sync_diff_list_state();
                            this.file_row_ranges.clear();
                            this.file_line_stats.clear();
                            this.selected_line_stats = LineStats::default();
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
