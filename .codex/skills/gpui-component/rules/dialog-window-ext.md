---
title: Use WindowExt for Dialog Management
impact: HIGH
tags: dialog, window-ext, overlay
---

## WindowExt Trait

WindowExt provides methods for managing dialogs, sheets, and notifications.

### WindowExt Methods

```rust
pub trait WindowExt: Sized {
    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static;

    fn close_dialog(&mut self, cx: &mut App);

    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    fn close_sheet(&mut self, cx: &mut App);

    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App);
}
```

### Opening a Dialog

```rust
Button::new("open")
    .label("Open Dialog")
    .on_click(|_, window, cx| {
        window.open_dialog(cx, |dialog, _, _| {
            dialog
                .title("Information")
                .child("This is a simple dialog.")
        });
    })
```

### Confirm Dialog

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .confirm()  // Adds OK/Cancel buttons, disables overlay close
        .title("Confirm Action")
        .child("Are you sure you want to proceed?")
        .on_ok(|_, window, cx| {
            // Handle confirm
            window.push_notification("Confirmed!", cx);
            true  // Return true to close dialog
        })
        .on_cancel(|_, window, cx| {
            // Handle cancel
            true  // Return true to close dialog
        })
});
```

### Alert Dialog

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .alert()  // Only OK button
        .title("Success")
        .child("Operation completed successfully.")
});
```

### Custom Button Props

```rust
window.open_dialog(cx, |dialog, _, cx| {
    dialog
        .confirm()
        .title("Warning")
        .child("This action cannot be undone.")
        .button_props(
            DialogButtonProps::default()
                .ok_text("Delete Permanently")
                .ok_variant(ButtonVariant::Danger)
                .cancel_text("Keep")
        )
});
```

### Opening a Sheet

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Settings")
        .placement(Placement::Right)
        .size(px(400.))
        .child(settings_content)
});
```

### Pushing Notifications

```rust
window.push_notification(
    Notification::new("File saved successfully")
        .variant(NotificationVariant::Success),
    cx
);

window.push_notification(
    Notification::new("Failed to save file")
        .variant(NotificationVariant::Error),
    cx
);
```

### Closing Programmatically

```rust
// Close top dialog
window.close_dialog(cx);

// Close sheet
window.close_sheet(cx);
```
