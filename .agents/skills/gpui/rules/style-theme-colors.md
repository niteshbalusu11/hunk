---
title: Always Use cx.theme() for Colors
impact: MEDIUM
tags: theme, colors, design-system
---

## Theme-Aware Colors

Never hardcode colors. Always use the theme system for consistency and dark mode support.

**Incorrect (hardcoded colors):**

```rust
// BAD: Hardcoded colors break themes
div()
    .bg(rgb(0xFFFFFF))
    .text_color(rgb(0x000000))
    .border_color(rgb(0xCCCCCC))
```

**Correct (use theme):**

```rust
// GOOD: Theme-aware colors
div()
    .bg(cx.theme().colors().background)
    .text_color(cx.theme().colors().text)
    .border_color(cx.theme().colors().border)
```

### Common Theme Colors

```rust
// Text
cx.theme().colors().text          // Primary text
cx.theme().colors().text_muted    // Secondary text
cx.theme().colors().text_disabled // Disabled text

// Backgrounds
cx.theme().colors().background          // Main background
cx.theme().colors().surface_background  // Panel background
cx.theme().colors().element_background  // Button/input background
cx.theme().colors().element_hover       // Hover state
cx.theme().colors().element_selected    // Selected state

// Borders
cx.theme().colors().border          // Default border
cx.theme().colors().border_focused  // Focus ring

// Status colors
cx.theme().status().error
cx.theme().status().warning
cx.theme().status().success
cx.theme().status().info
```

### Semantic Color Enum

```rust
pub enum Color {
    Default,         // Primary text
    Accent,          // Links, highlights
    Muted,           // Secondary text
    Error,
    Warning,
    Success,
    Info,
    Disabled,
    Custom(Hsla),
}

// Usage
let color = Color::Error.color(cx);
div().text_color(color)
```

### Button Variants Example

```rust
impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (bg, fg) = match self.variant {
            ButtonVariant::Primary => (
                cx.theme().colors().primary,
                cx.theme().colors().primary_foreground,
            ),
            ButtonVariant::Secondary => (
                cx.theme().colors().element_background,
                cx.theme().colors().text,
            ),
            ButtonVariant::Danger => (
                cx.theme().status().error,
                white(),
            ),
        };

        div()
            .bg(bg)
            .text_color(fg)
    }
}
```
