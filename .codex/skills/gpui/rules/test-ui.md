---
title: UI and Interaction Testing
impact: MEDIUM
tags: testing, ui, interaction, simulation
---

## UI Testing

GPUI supports testing rendering and user interactions.

### Drawing Elements

```rust
#[gpui::test]
async fn test_layout(cx: &mut TestAppContext) {
    let (view, mut cx) = cx.add_window_view(|window, cx| {
        MyView::new(window, cx)
    });

    // Draw element and get layout info
    let (layout, paint) = cx.draw(
        point(px(0.0), px(0.0)),        // Origin
        size(px(100.0), px(100.0)),     // Available space
        |window, cx| {
            div()
                .w(px(50.0))
                .h(px(30.0))
                .child("Hello")
        }
    );
}
```

### Debug Bounds

```rust
// In render code, mark elements for debugging
div()
    .debug("my-element")  // Marks element
    .child(content)

// In test, query bounds
#[gpui::test]
async fn test_bounds(cx: &mut TestAppContext) {
    let (view, mut cx) = cx.add_window_view(|w, cx| MyView::new(w, cx));

    cx.draw(...);

    if let Some(bounds) = cx.debug_bounds("my-element") {
        assert_eq!(bounds.size.width, px(100.0));
        assert_eq!(bounds.origin.x, px(0.0));
    }
}
```

### Simulating Mouse Events

```rust
#[gpui::test]
async fn test_click(cx: &mut TestAppContext) {
    let (view, mut cx) = cx.add_window_view(|w, cx| MyView::new(w, cx));

    // Simulate click
    cx.simulate_click(
        point(px(50.0), px(50.0)),
        Modifiers::none()
    );
    cx.run_until_parked();

    // Simulate click with modifiers
    cx.simulate_click(
        point(px(50.0), px(50.0)),
        Modifiers::command()
    );

    // Simulate mouse move
    cx.simulate_mouse_move(
        point(px(100.0), px(100.0)),
        None,
        Modifiers::none()
    );

    // Simulate drag
    cx.simulate_mouse_down(point(px(0.0), px(0.0)), Modifiers::none());
    cx.simulate_mouse_move(point(px(50.0), px(50.0)), Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(point(px(50.0), px(50.0)), Modifiers::none());
}
```

### Simulating Keyboard Events

```rust
#[gpui::test]
async fn test_keyboard(cx: &mut TestAppContext) {
    let (view, mut cx) = cx.add_window_view(|w, cx| MyView::new(w, cx));

    // Simulate keystroke
    cx.simulate_keystroke(Keystroke::parse("enter").unwrap());
    cx.run_until_parked();

    // Simulate with modifiers
    cx.simulate_keystroke(Keystroke::parse("cmd-s").unwrap());

    // Simulate text input
    cx.simulate_input("Hello, world!");
}
```

### Editor State Testing

```rust
#[gpui::test]
async fn test_editor(cx: &mut TestAppContext) {
    let mut cx = EditorTestContext::new(cx).await;

    // Set state with cursor (ˇ)
    cx.set_state("hello ˇworld");

    // Perform action
    cx.update_editor(|editor, window, cx| {
        editor.delete_word_backward(window, cx);
    });
    cx.run_until_parked();

    // Assert state
    cx.assert_editor_state("ˇworld");
}

// Selection markers
cx.set_state("«selected» text");
cx.assert_editor_state("«selected» text");

// Multiple cursors
cx.set_state("line1ˇ\nline2ˇ");
```
