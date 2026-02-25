---
title: Implement Standard Component Traits
impact: MEDIUM
tags: traits, disableable, selectable, sizable
---

## Component Traits System

Use standard traits for consistent component behavior across your UI.

### Core Traits

```rust
/// Disabled state
pub trait Disableable {
    fn disabled(self, disabled: bool) -> Self;
}

/// Selected state
pub trait Selectable: Sized {
    fn selected(self, selected: bool) -> Self;
    fn is_selected(&self) -> bool;
}

/// Size control
pub trait Sizable: Sized {
    fn with_size(self, size: impl Into<Size>) -> Self;

    fn xsmall(self) -> Self { self.with_size(Size::XSmall) }
    fn small(self) -> Self { self.with_size(Size::Small) }
    fn medium(self) -> Self { self.with_size(Size::Medium) }
    fn large(self) -> Self { self.with_size(Size::Large) }
}

/// Collapsible state
pub trait Collapsible {
    fn collapsed(self, collapsed: bool) -> Self;
    fn is_collapsed(&self) -> bool;
}
```

### Size Enum

```rust
#[derive(Clone, Default, Copy, PartialEq, Eq)]
pub enum Size {
    Size(Pixels),  // Custom size
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}
```

### Implementing Traits

```rust
impl Disableable for Button {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Sizable for Button {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Selectable for Checkbox {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}
```

### StyleSized for Size-Based Styling

```rust
pub trait StyleSized<T: Styled> {
    fn input_text_size(self, size: Size) -> Self;
    fn input_size(self, size: Size) -> Self;
    fn input_h(self, size: Size) -> Self;
}

impl<T: Styled> StyleSized<T> for T {
    fn input_h(self, size: Size) -> Self {
        match size {
            Size::Large => self.h_11(),
            Size::Medium => self.h_8(),
            Size::Small => self.h_6(),
            Size::XSmall => self.h_5(),
            _ => self.h_6(),
        }
    }

    fn input_text_size(self, size: Size) -> Self {
        match size {
            Size::XSmall => self.text_xs(),
            Size::Small | Size::Medium => self.text_sm(),
            Size::Large => self.text_base(),
            Size::Size(s) => self.text_size(s * 0.875),
        }
    }
}
```

### Usage Example

```rust
Button::new("submit", "Submit")
    .primary()          // ButtonVariants trait
    .large()            // Sizable trait
    .disabled(loading)  // Disableable trait

Checkbox::new("accept")
    .label("Accept terms")
    .selected(accepted) // Selectable trait
    .small()            // Sizable trait
```
