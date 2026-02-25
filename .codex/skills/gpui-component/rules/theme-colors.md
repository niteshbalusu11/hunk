---
title: Use ThemeColor for Consistent Colors
impact: HIGH
tags: theme, colors, consistency
---

## Theme Color System

Always use theme colors instead of hardcoded values for consistent theming.

### Theme Structure

```rust
pub struct Theme {
    pub colors: ThemeColor,
    pub mode: ThemeMode,
    pub font_family: SharedString,
    pub font_size: Pixels,
    pub radius: Pixels,
    pub radius_lg: Pixels,
    pub shadow: bool,
}

impl Global for Theme {}  // Stored as global state
```

### ThemeColor

```rust
pub struct ThemeColor {
    // Base colors
    pub background: Hsla,
    pub foreground: Hsla,
    pub border: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,

    // Primary colors
    pub primary: Hsla,
    pub primary_hover: Hsla,
    pub primary_active: Hsla,
    pub primary_foreground: Hsla,

    // Secondary colors
    pub secondary: Hsla,
    pub secondary_hover: Hsla,
    pub secondary_foreground: Hsla,

    // Semantic colors
    pub danger: Hsla,
    pub danger_hover: Hsla,
    pub danger_foreground: Hsla,
    pub warning: Hsla,
    pub success: Hsla,
    pub info: Hsla,

    // Component-specific colors
    pub input: Hsla,           // Input border
    pub ring: Hsla,            // Focus ring
    pub popover: Hsla,         // Popover background
    pub popover_foreground: Hsla,
    pub list_hover: Hsla,      // List item hover
    pub list_active: Hsla,     // List item active
    // ... 100+ color definitions
}
```

### ActiveTheme Trait

```rust
pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Theme {
        Theme::global(self)
    }
}
```

### Usage in Components

```rust
impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();

        let (bg, fg) = match self.variant {
            ButtonVariant::Primary => (theme.colors.primary, theme.colors.primary_foreground),
            ButtonVariant::Secondary => (theme.colors.secondary, theme.colors.secondary_foreground),
            ButtonVariant::Danger => (theme.colors.danger, theme.colors.danger_foreground),
            ButtonVariant::Ghost => (transparent(), theme.colors.foreground),
        };

        div()
            .bg(bg)
            .text_color(fg)
            .border_color(theme.colors.border)
            .rounded(theme.radius)
    }
}
```

### Common Theme Shortcuts

```rust
// In StyledExt
pub trait StyledExt: Styled + Sized {
    fn focused_border(self, cx: &App) -> Self {
        self.border_1().border_color(cx.theme().colors.ring)
    }

    fn popover_style(self, cx: &App) -> Self {
        self.bg(cx.theme().colors.popover)
            .text_color(cx.theme().colors.popover_foreground)
            .border_1()
            .border_color(cx.theme().colors.border)
            .shadow_lg()
            .rounded(cx.theme().radius)
    }
}
```

### Never Hardcode Colors

```rust
// BAD
div().bg(hsla(0.0, 0.0, 1.0, 1.0))  // Hardcoded white

// GOOD
div().bg(cx.theme().colors.background)
```
