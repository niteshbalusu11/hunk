impl DiffViewer {
    pub(super) fn open_workspace_text_context_menu(
        &mut self,
        target: WorkspaceTextContextMenuTarget,
        position: Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.workspace_text_context_menu = Some(WorkspaceTextContextMenuState { target, position });
        cx.notify();
    }

    pub(super) fn close_workspace_text_context_menu(&mut self, cx: &mut Context<Self>) {
        if self.workspace_text_context_menu.take().is_some() {
            cx.notify();
        }
    }

    pub(super) fn workspace_text_context_menu_copy(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(menu_state) = self.workspace_text_context_menu.as_ref() else {
            return;
        };
        match &menu_state.target {
            WorkspaceTextContextMenuTarget::FilesEditor(_) => {
                let Some(text) = self.files_editor.borrow().copy_selection_text() else {
                    return;
                };
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            WorkspaceTextContextMenuTarget::SelectableText(_)
            | WorkspaceTextContextMenuTarget::Terminal(_) => {
                let target_row_id = match &menu_state.target {
                    WorkspaceTextContextMenuTarget::SelectableText(target) => {
                        Some(target.row_id.as_str())
                    }
                    WorkspaceTextContextMenuTarget::Terminal(target) => Some(match target.kind {
                        WorkspaceTerminalKind::Ai => crate::app::AI_TERMINAL_TEXT_SELECTION_ROW_ID,
                        WorkspaceTerminalKind::Files => {
                            crate::app::FILES_TERMINAL_TEXT_SELECTION_ROW_ID
                        }
                    }),
                    _ => None,
                };
                let Some(selection_text) = target_row_id.and_then(|row_id| {
                    self.ai_text_selection.as_ref().and_then(|selection| {
                        (selection.row_id == row_id)
                            .then_some(selection)
                            .and_then(AiTextSelection::selected_text)
                    })
                }) else {
                    return;
                };
                cx.write_to_clipboard(ClipboardItem::new_string(selection_text));
            }
            WorkspaceTextContextMenuTarget::DiffRows(_) => {
                let Some(selection_text) = self.selected_rows_as_text() else {
                    return;
                };
                cx.write_to_clipboard(ClipboardItem::new_string(selection_text));
            }
        }
        self.close_workspace_text_context_menu(cx);
    }

    pub(super) fn workspace_text_context_menu_cut(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(WorkspaceTextContextMenuState {
            target: WorkspaceTextContextMenuTarget::FilesEditor(_),
            ..
        }) = self.workspace_text_context_menu.as_ref()
        else {
            return;
        };
        let Some(text) = self.files_editor.borrow_mut().cut_selection_text() else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.sync_editor_dirty_from_input(cx);
        self.close_workspace_text_context_menu(cx);
    }

    pub(super) fn workspace_text_context_menu_paste(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(menu_state) = self.workspace_text_context_menu.as_ref() else {
            return;
        };
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        match &menu_state.target {
            WorkspaceTextContextMenuTarget::FilesEditor(_) => {
                if self.files_editor.borrow_mut().paste_text(text.as_str()) {
                    self.sync_editor_dirty_from_input(cx);
                } else {
                    return;
                }
            }
            WorkspaceTextContextMenuTarget::Terminal(target) => {
                let pasted = match target.kind {
                    WorkspaceTerminalKind::Ai => self.ai_paste_terminal_from_clipboard(cx),
                    WorkspaceTerminalKind::Files => self.files_paste_terminal_from_clipboard(cx),
                };
                if !pasted {
                    return;
                }
            }
            WorkspaceTextContextMenuTarget::SelectableText(_)
            | WorkspaceTextContextMenuTarget::DiffRows(_) => return,
        }
        self.close_workspace_text_context_menu(cx);
        cx.notify();
    }

    pub(super) fn workspace_text_context_menu_select_all(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(menu_state) = self.workspace_text_context_menu.clone() else {
            return;
        };
        match menu_state.target {
            WorkspaceTextContextMenuTarget::FilesEditor(_) => {
                if !self.files_editor.borrow_mut().select_all_action() {
                    return;
                }
                self.sync_editor_dirty_from_input(cx);
            }
            WorkspaceTextContextMenuTarget::SelectableText(target) => {
                if !self.ai_select_all_text_for_surfaces(
                    target.row_id.as_str(),
                    target.selection_surfaces,
                    cx,
                ) {
                    return;
                }
            }
            WorkspaceTextContextMenuTarget::Terminal(target) => {
                let row_id = match target.kind {
                    WorkspaceTerminalKind::Ai => crate::app::AI_TERMINAL_TEXT_SELECTION_ROW_ID,
                    WorkspaceTerminalKind::Files => crate::app::FILES_TERMINAL_TEXT_SELECTION_ROW_ID,
                };
                if !self.ai_select_all_text_for_surfaces(
                    row_id,
                    target.selection_surfaces,
                    cx,
                ) {
                    return;
                }
            }
            WorkspaceTextContextMenuTarget::DiffRows(_) => {
                self.select_all_rows(cx);
            }
        }
        self.close_workspace_text_context_menu(cx);
    }

    pub(super) fn workspace_text_context_menu_clear_terminal(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(WorkspaceTextContextMenuState {
            target: WorkspaceTextContextMenuTarget::Terminal(target),
            ..
        }) = self.workspace_text_context_menu.as_ref()
        else {
            return;
        };
        match target.kind {
            WorkspaceTerminalKind::Ai => self.ai_clear_terminal_session_action(cx),
            WorkspaceTerminalKind::Files => self.files_clear_terminal_session_action(cx),
        }
        self.close_workspace_text_context_menu(cx);
    }

    pub(super) fn workspace_text_context_menu_open_link(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(WorkspaceTextContextMenuState {
            target: WorkspaceTextContextMenuTarget::SelectableText(target),
            ..
        }) = self.workspace_text_context_menu.as_ref()
        else {
            return;
        };
        let Some(raw_target) = target.link_target.clone() else {
            return;
        };
        self.activate_markdown_link(raw_target, None, cx);
        self.close_workspace_text_context_menu(cx);
    }
}
