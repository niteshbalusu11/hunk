---
title: Use Root Component as Window Root
impact: CRITICAL
tags: dialog, root, overlay
---

## Root Pattern

gpui-component uses `Root` as the window root to manage all overlay layers.

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
│  └── Render Layers                                          │
│      ├── Layer 0: Root.view (main content)                  │
│      ├── Layer 1: Sheet (side drawer)                       │
│      ├── Layer 2: Dialogs (stackable)                       │
│      └── Layer 3: Notifications                             │
└─────────────────────────────────────────────────────────────┘
```

### Application Setup

```rust
fn main() {
    App::new().run(|cx: &mut App| {
        // Initialize gpui-component
        gpui_component::init(cx);

        cx.open_window(options, |window, cx| {
            // Create your main view
            let main_view = cx.new(|cx| MyApp::new(window, cx));

            // MUST wrap in Root for dialog support
            cx.new(|cx| Root::new(main_view, window, cx))
        });
    });
}
```

### Root Structure

```rust
pub struct Root {
    pub(crate) active_sheet: Option<ActiveSheet>,
    pub(crate) active_dialogs: Vec<ActiveDialog>,
    pub notification: Entity<NotificationList>,
    view: AnyView,
}
```

### Why Root is Required

Without Root:
- `window.open_dialog()` won't work
- `window.open_sheet()` won't work
- `window.push_notification()` won't work
- No overlay management

### Init Function

```rust
// lib.rs of gpui-component
pub fn init(cx: &mut App) {
    theme::init(cx);
    root::init(cx);
    dialog::init(cx);
    input::init(cx);
    // ... other component inits
}
```

### Component Init Registers KeyBindings

```rust
// dialog::init
const CONTEXT: &str = "Dialog";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
    ]);
}
```
