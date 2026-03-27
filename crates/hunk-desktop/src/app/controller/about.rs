impl DiffViewer {
    pub(super) fn open_about_hunk_action(
        &mut self,
        _: &AboutHunk,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        gpui_component::WindowExt::open_alert_dialog(window, cx, move |alert, _, cx| {
            alert
                .width(px(420.0))
                .title("About Hunk")
                .description(ABOUT_HUNK_VERSION_LABEL)
                .button_props(
                    gpui_component::dialog::DialogButtonProps::default().ok_text("Close"),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().foreground)
                                .child(ABOUT_HUNK_DESCRIPTION_LINE_ONE),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(ABOUT_HUNK_DESCRIPTION_LINE_TWO),
                        ),
                )
        });
    }
}
