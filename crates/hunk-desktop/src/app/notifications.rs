use gpui::{ClipboardItem, SharedString};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    notification::Notification,
};

fn with_copy_action(notification: Notification, message: SharedString) -> Notification {
    notification
        .action(move |_, _, _| {
            Button::new("copy-notification")
                .label("Copy")
                .ghost()
                .on_click({
                    let message = message.clone();
                    move |_, _, cx| {
                        cx.stop_propagation();
                        cx.write_to_clipboard(ClipboardItem::new_string(message.to_string()));
                    }
                })
        })
        .autohide(true)
}

pub(crate) fn success(message: impl Into<SharedString>) -> Notification {
    let message = message.into();
    with_copy_action(Notification::success(message.clone()), message)
}

pub(crate) fn error(message: impl Into<SharedString>) -> Notification {
    let message = message.into();
    with_copy_action(Notification::error(message.clone()), message)
}

pub(crate) fn warning(message: impl Into<SharedString>) -> Notification {
    let message = message.into();
    with_copy_action(Notification::warning(message.clone()), message)
}
