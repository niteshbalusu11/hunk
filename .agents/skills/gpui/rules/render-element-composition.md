---
title: Build Element Trees with Method Chaining
impact: CRITICAL
tags: element, div, composition, children
---

## Element Composition

GPUI uses method chaining to build element trees, similar to SwiftUI or Jetpack Compose.

**Basic Structure:**

```rust
div()
    .flex()                    // Display: flex
    .flex_col()               // Flex direction: column
    .gap_2()                  // Gap: 0.5rem
    .p_4()                    // Padding: 1rem
    .bg(cx.theme().colors().background)
    .child(header)            // Add single child
    .children(items)          // Add multiple children
```

### Adding Children

**Single child:**
```rust
div()
    .child(Button::new("submit").label("Submit"))
```

**Multiple children:**
```rust
div()
    .children(vec![
        div().child("Item 1").into_any_element(),
        div().child("Item 2").into_any_element(),
    ])
```

**Conditional children:**
```rust
div()
    .when_some(self.icon, |this, icon| {
        this.child(icon)
    })
    .child(label)
```

### Nested Composition

```rust
div()
    .v_flex()
    .gap_4()
    .child(
        div()
            .h_flex()
            .justify_between()
            .child(title)
            .child(close_button)
    )
    .child(
        div()
            .flex_1()
            .overflow_y_scroll()
            .child(content)
    )
    .child(
        div()
            .h_flex()
            .justify_end()
            .gap_2()
            .child(cancel_button)
            .child(confirm_button)
    )
```

### Using SharedString

Use `SharedString` for text to avoid copying:

```rust
// SharedString is either &'static str or Arc<str>
let label: SharedString = "Hello".into();
let dynamic_label: SharedString = format!("Count: {}", count).into();

div().child(label)
```
