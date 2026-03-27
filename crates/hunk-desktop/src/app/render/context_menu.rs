impl DiffViewer {
    fn render_workspace_text_context_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu_state = self.workspace_text_context_menu.as_ref()?.clone();
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();

        Some(
            deferred(
                anchored()
                    .position(menu_state.position)
                    .anchor(Corner::TopLeft)
                    .snap_to_window_with_margin(px(8.0))
                    .child(
                        v_flex()
                            .id("workspace-text-context-menu")
                            .w(px(220.0))
                            .p_1()
                            .gap_0p5()
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(hunk_opacity(cx.theme().border, is_dark, 0.92, 0.74))
                            .bg(cx.theme().popover)
                            .shadow_none()
                            .on_mouse_down_out({
                                let view = view.clone();
                                move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.close_workspace_text_context_menu(cx);
                                    });
                                }
                            })
                            .children(self.render_workspace_text_context_menu_entries(
                                view,
                                &menu_state.target,
                                cx,
                            )),
                    ),
            )
            .into_any_element(),
        )
    }

    fn render_workspace_text_context_menu_entries(
        &self,
        view: Entity<Self>,
        target: &WorkspaceTextContextMenuTarget,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut items = Vec::new();
        match target {
            WorkspaceTextContextMenuTarget::FilesEditor(target) => {
                items.push(
                    self.render_workspace_text_context_menu_item("Cut", target.can_cut, {
                        let view = view.clone();
                        move |cx| {
                            view.update(cx, |this, cx| {
                                this.workspace_text_context_menu_cut(cx);
                            });
                        }
                    }, cx),
                );
                items.push(
                    self.render_workspace_text_context_menu_item("Copy", target.can_copy, {
                        let view = view.clone();
                        move |cx| {
                            view.update(cx, |this, cx| {
                                this.workspace_text_context_menu_copy(cx);
                            });
                        }
                    }, cx),
                );
                items.push(
                    self.render_workspace_text_context_menu_item("Paste", target.can_paste, {
                        let view = view.clone();
                        move |cx| {
                            view.update(cx, |this, cx| {
                                this.workspace_text_context_menu_paste(cx);
                            });
                        }
                    }, cx),
                );
                items.push(div().h(px(1.0)).mx_1().bg(cx.theme().border).into_any_element());
                items.push(
                    self.render_workspace_text_context_menu_item(
                        "Select All",
                        target.can_select_all,
                        {
                            let view = view.clone();
                            move |cx| {
                                view.update(cx, |this, cx| {
                                    this.workspace_text_context_menu_select_all(cx);
                                });
                            }
                        },
                        cx,
                    ),
                );
            }
            WorkspaceTextContextMenuTarget::SelectableText(target) => {
                items.push(
                    self.render_workspace_text_context_menu_item("Copy", target.can_copy, {
                        let view = view.clone();
                        move |cx| {
                            view.update(cx, |this, cx| {
                                this.workspace_text_context_menu_copy(cx);
                            });
                        }
                    }, cx),
                );
                items.push(
                    self.render_workspace_text_context_menu_item(
                        "Select All",
                        target.can_select_all,
                        {
                            let view = view.clone();
                            move |cx| {
                                view.update(cx, |this, cx| {
                                    this.workspace_text_context_menu_select_all(cx);
                                });
                            }
                        },
                        cx,
                    ),
                );
                if target.link_target.is_some() {
                    items.push(div().h(px(1.0)).mx_1().bg(cx.theme().border).into_any_element());
                    items.push(
                        self.render_workspace_text_context_menu_item("Open Link", true, {
                            let view = view.clone();
                            move |cx| {
                                view.update(cx, |this, cx| {
                                    this.workspace_text_context_menu_open_link(cx);
                                });
                            }
                        }, cx),
                    );
                }
            }
            WorkspaceTextContextMenuTarget::Terminal(target) => {
                items.push(
                    self.render_workspace_text_context_menu_item("Copy", target.can_copy, {
                        let view = view.clone();
                        move |cx| {
                            view.update(cx, |this, cx| {
                                this.workspace_text_context_menu_copy(cx);
                            });
                        }
                    }, cx),
                );
                items.push(
                    self.render_workspace_text_context_menu_item("Paste", target.can_paste, {
                        let view = view.clone();
                        move |cx| {
                            view.update(cx, |this, cx| {
                                this.workspace_text_context_menu_paste(cx);
                            });
                        }
                    }, cx),
                );
                items.push(
                    self.render_workspace_text_context_menu_item(
                        "Select All",
                        target.can_select_all,
                        {
                            let view = view.clone();
                            move |cx| {
                                view.update(cx, |this, cx| {
                                    this.workspace_text_context_menu_select_all(cx);
                                });
                            }
                        },
                        cx,
                    ),
                );
                items.push(div().h(px(1.0)).mx_1().bg(cx.theme().border).into_any_element());
                items.push(
                    self.render_workspace_text_context_menu_item("Clear", target.can_clear, {
                        let view = view.clone();
                        move |cx| {
                            view.update(cx, |this, cx| {
                                this.workspace_text_context_menu_clear_terminal(cx);
                            });
                        }
                    }, cx),
                );
            }
            WorkspaceTextContextMenuTarget::DiffRows(target) => {
                items.push(
                    self.render_workspace_text_context_menu_item("Copy", target.can_copy, {
                        let view = view.clone();
                        move |cx| {
                            view.update(cx, |this, cx| {
                                this.workspace_text_context_menu_copy(cx);
                            });
                        }
                    }, cx),
                );
                items.push(
                    self.render_workspace_text_context_menu_item(
                        "Select All",
                        target.can_select_all,
                        move |cx| {
                            view.update(cx, |this, cx| {
                                this.workspace_text_context_menu_select_all(cx);
                            });
                        },
                        cx,
                    ),
                );
            }
        }
        items
    }

    fn render_workspace_text_context_menu_item(
        &self,
        label: &'static str,
        enabled: bool,
        on_click: impl Fn(&mut App) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let text_color = if enabled {
            cx.theme().popover_foreground
        } else {
            cx.theme().muted_foreground
        };
        let hover_bg = cx.theme().secondary_hover;
        div()
            .w_full()
            .px_2()
            .py_1()
            .rounded(px(6.0))
            .text_sm()
            .text_color(text_color)
            .when(enabled, |this| {
                this.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    cx.stop_propagation();
                    on_click(cx);
                })
                .hover(move |style| style.bg(hover_bg).cursor_pointer())
            })
            .child(div().min_w_0().truncate().child(label))
            .into_any_element()
    }
}
