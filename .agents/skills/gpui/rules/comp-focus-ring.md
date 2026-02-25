---
title: Add Focus Ring for Accessibility
impact: MEDIUM
tags: focus, accessibility, a11y
---

## Focus Ring Pattern

Focus rings indicate keyboard focus for accessibility. Always show them on focusable elements.

### FocusableExt Trait

```rust
pub(crate) trait FocusableExt<T: ParentElement + Styled + Sized> {
    fn focus_ring(
        self,
        is_focused: bool,
        margins: Pixels,
        window: &Window,
        cx: &App
    ) -> Self;
}
```

### Implementation

```rust
impl<T: ParentElement + Styled + Sized> FocusableExt<T> for T {
    fn focus_ring(
        mut self,
        is_focused: bool,
        margins: Pixels,
        window: &Window,
        cx: &App
    ) -> Self {
        if !is_focused {
            return self;
        }

        // Add focus ring as overlay child
        self.child(
            div()
                .absolute()
                .inset(-(RING_BORDER_WIDTH + margins))
                .border(RING_BORDER_WIDTH)
                .border_color(cx.theme().ring.alpha(0.2))
                .rounded(/* calculate based on element */)
        )
    }
}
```

### Usage in Components

```rust
impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Get or create FocusHandle (persist with keyed state)
        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();

        let is_focused = focus_handle.is_focused(window);

        div()
            .relative()  // Required for absolute focus ring
            .when(!self.disabled, |this| {
                this.track_focus(
                    &focus_handle
                        .tab_index(self.tab_index)
                        .tab_stop(self.tab_stop)
                )
            })
            // ... other styles
            .focus_ring(is_focused, px(0.), window, cx)
    }
}
```

### Alternative: Border-Based Focus

```rust
div()
    .border_1()
    .border_color(if is_focused {
        cx.theme().colors().border_focused
    } else {
        cx.theme().colors().border
    })
```

### Focus Within

For containers that should show focus when children are focused:

```rust
div()
    .when(focus_handle.contains_focused(window, cx), |this| {
        this.border_color(cx.theme().colors().border_focused)
    })
```
