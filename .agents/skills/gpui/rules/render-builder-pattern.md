---
title: Design Components with Builder Pattern
impact: HIGH
tags: builder, component, api-design
---

## Builder Pattern for Components

Design components with a fluent builder API for ergonomic usage.

**Component Structure:**

```rust
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    icon: Option<IconName>,
    variant: ButtonVariant,
    size: Size,
    disabled: bool,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl Button {
    // Constructor with required fields
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            variant: ButtonVariant::default(),
            size: Size::default(),
            disabled: false,
            on_click: None,
        }
    }

    // Builder methods return Self
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}
```

**Usage:**

```rust
Button::new("submit", "Submit")
    .icon(IconName::Check)
    .variant(ButtonVariant::Primary)
    .disabled(is_loading)
    .on_click(|_, window, cx| {
        // Handle click
    })
```

### Convenience Methods via Traits

```rust
pub trait ButtonVariants: Sized {
    fn with_variant(self, variant: ButtonVariant) -> Self;

    fn primary(self) -> Self { self.with_variant(ButtonVariant::Primary) }
    fn danger(self) -> Self { self.with_variant(ButtonVariant::Danger) }
    fn ghost(self) -> Self { self.with_variant(ButtonVariant::Ghost) }
}

impl ButtonVariants for Button {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}

// Now you can write:
Button::new("delete", "Delete").danger()
```

### Implementing Styled Trait

```rust
impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

// Now Button supports all styling methods:
Button::new("btn", "Click")
    .bg(red_500())
    .rounded_lg()
    .p_4()
```
