---
title: Use .when() and .when_some() for Conditional Styling
impact: HIGH
tags: conditional, when, styling
---

## Conditional Styling with FluentBuilder

GPUI provides fluent methods for conditional styling and rendering.

### Basic Conditions

**Incorrect (verbose if-else):**

```rust
// BAD: Verbose and hard to chain
let mut element = div().flex();
if is_active {
    element = element.bg(active_color);
}
if is_disabled {
    element = element.opacity(0.5);
}
```

**Correct (use .when()):**

```rust
// GOOD: Fluent conditional styling
div()
    .flex()
    .when(is_active, |this| {
        this.bg(cx.theme().colors().element_selected)
            .border_color(cx.theme().colors().border_focused)
    })
    .when(is_disabled, |this| {
        this.opacity(0.5)
            .cursor_not_allowed()
    })
```

### Option Handling

```rust
// when_some - apply if Some
div()
    .when_some(self.custom_width, |this, width| this.w(width))
    .when_some(self.icon, |this, icon| this.child(icon))

// when_none - apply if None
div()
    .when_none(&self.content, |this| {
        this.child("No content")
    })
```

### If-Else Pattern

```rust
div()
    .when_else(
        is_selected,
        |this| this.bg(selected_color).text_color(white),
        |this| this.bg(default_color).text_color(default_text)
    )
```

### Map for Always-Apply

```rust
// map() always applies, useful for complex transformations
div()
    .map(|this| {
        this.child(header)
            .children(items.iter().map(|item| render_item(item)))
    })
```

### Per-Corner Rounding

```rust
div()
    .when(rounding.top_left, |this| this.rounded_tl_md())
    .when(rounding.top_right, |this| this.rounded_tr_md())
    .when(rounding.bottom_left, |this| this.rounded_bl_md())
    .when(rounding.bottom_right, |this| this.rounded_br_md())
```
