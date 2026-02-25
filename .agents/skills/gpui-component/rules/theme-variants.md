---
title: Implement Variant Enums for Component Styles
impact: HIGH
tags: theme, variants, styling
---

## Variant Pattern

Use variant enums to provide predefined style variations.

### ButtonVariant Example

```rust
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    Primary,
    #[default]
    Secondary,
    Danger,
    Warning,
    Success,
    Info,
    Ghost,
    Link,
    Text,
    Custom(ButtonCustomVariant),
}
```

### Variant Style Resolution

```rust
impl ButtonVariant {
    fn normal(&self, outline: bool, cx: &App) -> ButtonVariantStyle {
        let theme = cx.theme();
        match self {
            ButtonVariant::Primary => ButtonVariantStyle {
                bg: if outline { transparent() } else { theme.colors.primary },
                border: theme.colors.primary,
                fg: if outline { theme.colors.primary } else { theme.colors.primary_foreground },
                underline: false,
                shadow: theme.shadow,
            },
            ButtonVariant::Danger => ButtonVariantStyle {
                bg: if outline { transparent() } else { theme.colors.danger },
                border: theme.colors.danger,
                fg: if outline { theme.colors.danger } else { theme.colors.danger_foreground },
                underline: false,
                shadow: theme.shadow,
            },
            ButtonVariant::Ghost => ButtonVariantStyle {
                bg: transparent(),
                border: transparent(),
                fg: theme.colors.foreground,
                underline: false,
                shadow: false,
            },
            // ... other variants
        }
    }

    fn hovered(&self, outline: bool, cx: &App) -> ButtonVariantStyle { ... }
    fn active(&self, outline: bool, cx: &App) -> ButtonVariantStyle { ... }
    fn disabled(&self, outline: bool, cx: &App) -> ButtonVariantStyle { ... }
}

struct ButtonVariantStyle {
    bg: Hsla,
    border: Hsla,
    fg: Hsla,
    underline: bool,
    shadow: bool,
}
```

### ButtonVariants Trait

```rust
pub trait ButtonVariants: Sized {
    fn with_variant(self, variant: ButtonVariant) -> Self;

    fn primary(self) -> Self { self.with_variant(ButtonVariant::Primary) }
    fn secondary(self) -> Self { self.with_variant(ButtonVariant::Secondary) }
    fn danger(self) -> Self { self.with_variant(ButtonVariant::Danger) }
    fn warning(self) -> Self { self.with_variant(ButtonVariant::Warning) }
    fn success(self) -> Self { self.with_variant(ButtonVariant::Success) }
    fn ghost(self) -> Self { self.with_variant(ButtonVariant::Ghost) }
    fn link(self) -> Self { self.with_variant(ButtonVariant::Link) }
}

impl ButtonVariants for Button {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}
```

### Usage in Render

```rust
impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = self.variant.normal(self.outline, cx);

        div()
            .bg(style.bg)
            .text_color(style.fg)
            .border_color(style.border)
            .when(style.shadow, |this| this.shadow_sm())
            .when(style.underline, |this| this.underline())
            // Hover state
            .hover(|this| {
                let hover_style = self.variant.hovered(self.outline, cx);
                this.bg(hover_style.bg).text_color(hover_style.fg)
            })
    }
}
```

### Usage

```rust
Button::new("save").label("Save").primary()
Button::new("delete").label("Delete").danger()
Button::new("cancel").label("Cancel").ghost()
Button::new("learn-more").label("Learn more").link()

// With outline
Button::new("save").label("Save").primary().outline()
```
