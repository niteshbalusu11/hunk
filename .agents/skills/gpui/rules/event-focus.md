---
title: Manage Focus with FocusHandle
impact: HIGH
tags: focus, keyboard, accessibility
---

## Focus Management

FocusHandle controls keyboard focus and enables action dispatch.

### Creating and Using FocusHandle

```rust
struct Editor {
    focus_handle: FocusHandle,
}

impl Editor {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for Editor {
    fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }
}
```

### Focus in Rendering

```rust
impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("Editor")  // Required for action dispatch
            .track_focus(&self.focus_handle)
            .on_focus(cx.listener(|this, event, window, cx| {
                this.on_focus_gained(cx);
            }))
            .on_blur(cx.listener(|this, event, window, cx| {
                this.on_focus_lost(cx);
            }))
            .on_action(cx.listener(Self::move_up))
    }
}
```

### Programmatic Focus

```rust
// Focus an element
self.focus_handle.focus(window, cx);

// Check focus state
if self.focus_handle.is_focused(window) {
    // Element has focus
}

// Check if focus is within subtree
if self.focus_handle.contains_focused(window, cx) {
    // Focus is on this element or a descendant
}
```

### Focus Handle with Tab Navigation

```rust
// Configure tab behavior
let focus_handle = self.focus_handle
    .tab_index(0)      // Tab order (0 = natural order)
    .tab_stop(true);   // Can receive focus via Tab

div()
    .track_focus(&focus_handle)
    .child(content)
```

### RenderOnce Components with Focus

For RenderOnce components, use keyed state to persist FocusHandle:

```rust
impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();

        let is_focused = focus_handle.is_focused(window);

        div()
            .when(!self.disabled, |this| {
                this.track_focus(&focus_handle)
            })
            .when(is_focused, |this| {
                this.border_color(cx.theme().colors().border_focused)
            })
    }
}
```
