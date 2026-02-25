---
title: Use cx.listener() for View-Bound Event Handlers
impact: HIGH
tags: listener, events, click, mouse
---

## Event Handlers with cx.listener()

Use `cx.listener()` when you need access to the view's state in event handlers.

**Incorrect (closure doesn't have view access):**

```rust
// BAD: Can't access self in closure
impl Render for Counter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().on_click(|event, window, cx| {
            // Can't access self.count here!
        })
    }
}
```

**Correct (use cx.listener):**

```rust
// GOOD: cx.listener automatically binds to the view
impl Render for Counter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .on_click(cx.listener(|this: &mut Counter, event, window, cx| {
                this.count += 1;
                cx.notify();
            }))
    }
}
```

### Mouse Events

```rust
div()
    .on_click(cx.listener(Self::handle_click))
    .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_mouse_down))
    .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
    .on_mouse_move(cx.listener(Self::handle_mouse_move))
    .on_scroll_wheel(cx.listener(Self::handle_scroll))
```

### Direct Closures (Without Self Access)

When you don't need view state, use direct closures:

```rust
div()
    .on_click(|event: &ClickEvent, window, cx| {
        // Stateless handler
        println!("Clicked at {:?}", event.position);
    })
```

### Event Handler Methods

```rust
impl MyView {
    fn handle_click(
        &mut self,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected = !self.selected;
        cx.notify();
    }

    fn handle_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.scroll_offset += event.delta.y;
        cx.notify();
    }
}
```

### Using listener_for with Entities

For RenderOnce components that need to call methods on an Entity:

```rust
impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .on_action(window.listener_for(&self.state, InputState::backspace))
            .on_action(window.listener_for(&self.state, InputState::delete))
    }
}
```
