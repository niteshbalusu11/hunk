---
title: Implement Styled for Style Customization
impact: CRITICAL
tags: trait, styled, customization
---

## Styled Trait

Implement `Styled` to allow users to customize component styles with method chaining.

### Implementation

```rust
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    style: StyleRefinement,  // Store style overrides
    // ...
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
```

### What This Enables

```rust
// Users can now write:
Button::new("btn")
    .label("Click me")
    .bg(red_500())           // Background color
    .text_color(white())     // Text color
    .p_4()                   // Padding
    .rounded_lg()            // Border radius
    .border_1()              // Border width
    .border_color(gray_300()) // Border color
    .shadow_md()             // Shadow
```

### Applying Styles in Render

```rust
impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            // Apply stored style overrides
            .style(self.style)
            // Then apply component's base styles
            .bg(cx.theme().primary)
            .rounded(cx.theme().radius)
    }
}
```

### Order Matters

Styles applied later override earlier ones:

```rust
// In RenderOnce::render
div()
    .style(self.style)    // User overrides applied first
    .bg(default_bg)       // This will OVERRIDE user's .bg()!

// Correct order:
div()
    .bg(default_bg)       // Base styles first
    .style(self.style)    // User overrides applied last
```

### StyledExt for Common Patterns

```rust
pub trait StyledExt: Styled + Sized {
    fn h_flex(self) -> Self {
        self.flex().flex_row().items_center()
    }

    fn v_flex(self) -> Self {
        self.flex().flex_col()
    }

    fn debug_red(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(red_500())
        } else {
            self
        }
    }
}

// Auto-implement for all Styled types
impl<T: Styled + Sized> StyledExt for T {}
```
