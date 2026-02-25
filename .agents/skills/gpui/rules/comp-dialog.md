---
title: Use WindowExt for Dialog Management
impact: MEDIUM
tags: dialog, modal, popover, window
---

## Dialog and Overlay Management

Use the Root pattern with WindowExt for managing dialogs and overlays.

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Window                                                      │
│  ├── Root (Entity, manages overlay state)                   │
│  │   ├── view: AnyView (user's main view)                   │
│  │   ├── active_dialogs: Vec<ActiveDialog>                  │
│  │   ├── active_sheet: Option<ActiveSheet>                  │
│  │   └── notification: Entity<NotificationList>             │
│  │                                                          │
│  └── Render layers                                          │
│      ├── Layer 0: Root.view (main content)                  │
│      ├── Layer 1: Sheet (side panel)                        │
│      ├── Layer 2: Dialogs (can stack)                       │
│      └── Layer 3: Notifications                             │
└─────────────────────────────────────────────────────────────┘
```

### WindowExt Trait

```rust
pub trait WindowExt: Sized {
    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static;

    fn close_dialog(&mut self, cx: &mut App);

    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App);
}
```

### Opening Dialogs

```rust
// Simple information dialog
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("Information")
        .child("This is a simple dialog.")
});

// Confirmation dialog
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .confirm()  // Adds OK/Cancel buttons
        .title("Confirm Action")
        .child("Are you sure?")
        .on_ok(|_, window, cx| {
            // Handle confirmation
            true  // Return true to close
        })
        .on_cancel(|_, window, cx| {
            // Handle cancellation
            true
        })
});

// Alert dialog
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .alert()  // Only OK button
        .title("Success")
        .child("Operation completed.")
});
```

### Custom Dialog Footer

```rust
window.open_dialog(cx, move |dialog, _, _| {
    dialog
        .title("Custom Footer")
        .child(content)
        .footer(move |render_ok, render_cancel, window, cx| {
            vec![
                render_cancel(window, cx),
                Button::new("preview")
                    .label("Preview")
                    .ghost()
                    .into_any_element(),
                render_ok(window, cx),
            ]
        })
});
```

### Notifications

```rust
// Success notification
window.push_notification(
    Notification::new("Operation completed")
        .variant(NotificationVariant::Success),
    cx
);

// Error notification
window.push_notification(
    Notification::new("Failed to save")
        .variant(NotificationVariant::Error),
    cx
);
```

### Sheet (Side Panel)

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Details")
        .size(px(400.))
        .placement(Placement::Right)  // Left, Right, Top, Bottom
        .child(details_content)
});
```
