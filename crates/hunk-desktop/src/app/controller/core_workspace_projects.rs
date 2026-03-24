impl DiffViewer {
    fn update_project_picker_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active_project_path = self
            .project_path
            .as_deref()
            .or(self.state.active_project_path().map(std::path::PathBuf::as_path));
        let delegate = build_project_picker_delegate(
            self.state.workspace_project_paths.as_slice(),
            active_project_path,
        );
        let selected_index = project_picker_selected_index(
            self.state.workspace_project_paths.as_slice(),
            active_project_path,
        );
        Self::set_index_picker_state(
            &self.project_picker_state,
            delegate,
            selected_index,
            window,
            cx,
        );
        cx.notify();
    }

    fn sync_project_picker_state(&mut self, cx: &mut Context<Self>) {
        let active_project_path = self
            .project_path
            .as_deref()
            .or(self.state.active_project_path().map(std::path::PathBuf::as_path));
        let project_picker_state = self.project_picker_state.clone();
        let delegate = build_project_picker_delegate(
            self.state.workspace_project_paths.as_slice(),
            active_project_path,
        );
        let selected_index = project_picker_selected_index(
            self.state.workspace_project_paths.as_slice(),
            active_project_path,
        );

        Self::sync_index_picker_state(
            project_picker_state,
            delegate,
            selected_index,
            "failed to sync project picker state",
            cx,
        );
    }
}
