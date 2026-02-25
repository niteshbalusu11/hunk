---
title: Define and Register Actions for Keyboard Shortcuts
impact: HIGH
tags: actions, keyboard, shortcuts
---

## Actions System

Actions are commands that can be triggered by keyboard shortcuts or programmatically.

### Defining Actions

```rust
// Simple actions (no data)
actions!(editor, [
    MoveUp,
    MoveDown,
    Delete,
    Save,
]);

// Actions with data
#[derive(Clone, PartialEq, Action)]
pub struct GoToLine {
    pub line: u32,
}
```

### Registering Action Handlers

```rust
impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("Editor")  // Required for action dispatch
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::go_to_line))
            .child(self.content.clone())
    }
}

impl Editor {
    fn move_up(&mut self, _action: &MoveUp, window: &mut Window, cx: &mut Context<Self>) {
        self.cursor.move_up();
        cx.notify();
    }

    fn go_to_line(&mut self, action: &GoToLine, window: &mut Window, cx: &mut Context<Self>) {
        self.cursor.go_to_line(action.line);
        cx.notify();
    }
}
```

### Binding Keyboard Shortcuts

```rust
// In app initialization
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", MoveUp, Some("Editor")),
        KeyBinding::new("down", MoveDown, Some("Editor")),
        KeyBinding::new("ctrl-g", GoToLine { line: 0 }, Some("Editor")),
        KeyBinding::new("escape", Cancel, Some("Dialog")),
    ]);
}
```

### Dispatching Actions Programmatically

```rust
// Via window
window.dispatch_action(MoveUp.boxed_clone(), cx);

// Via focus handle
self.focus_handle.dispatch_action(&MoveUp, window, cx);

// With data
self.focus_handle.dispatch_action(&GoToLine { line: 42 }, window, cx);
```

### Key Context Nesting

```rust
div()
    .key_context("Editor")  // Parent context
    .child(
        div()
            .key_context("Editor::LineNumbers")  // Nested context
            // Actions registered here take precedence
    )
```
