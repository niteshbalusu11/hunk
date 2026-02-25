---
title: Implement Sizable with Size Enum
impact: HIGH
tags: trait, sizable, size
---

## Sizable Trait

Use `Size` enum and `Sizable` trait for consistent component sizing.

### Size Enum

```rust
#[derive(Clone, Default, Copy, PartialEq, Eq)]
pub enum Size {
    Size(Pixels),  // Custom pixel size
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}

impl From<Pixels> for Size {
    fn from(pixels: Pixels) -> Self {
        Size::Size(pixels)
    }
}
```

### Sizable Trait

```rust
pub trait Sizable: Sized {
    fn with_size(self, size: impl Into<Size>) -> Self;

    fn xsmall(self) -> Self { self.with_size(Size::XSmall) }
    fn small(self) -> Self { self.with_size(Size::Small) }
    fn medium(self) -> Self { self.with_size(Size::Medium) }
    fn large(self) -> Self { self.with_size(Size::Large) }
}
```

### Implementation

```rust
#[derive(IntoElement)]
pub struct Button {
    size: Size,
    // ...
}

impl Sizable for Button {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
```

### StyleSized Trait

Apply size-based styles to elements:

```rust
pub trait StyleSized<T: Styled> {
    fn input_text_size(self, size: Size) -> Self;
    fn input_size(self, size: Size) -> Self;  // px + py + h
    fn input_h(self, size: Size) -> Self;
    fn button_text_size(self, size: Size) -> Self;
}

impl<T: Styled> StyleSized<T> for T {
    fn input_h(self, size: Size) -> Self {
        match size {
            Size::Large => self.h_11(),
            Size::Medium => self.h_8(),
            Size::Small => self.h_6(),
            Size::XSmall => self.h_5(),
            Size::Size(px) => self.h(px),
        }
    }

    fn input_text_size(self, size: Size) -> Self {
        match size {
            Size::XSmall => self.text_xs(),
            Size::Small => self.text_sm(),
            Size::Medium => self.text_sm(),
            Size::Large => self.text_base(),
            Size::Size(size) => self.text_size(size * 0.875),
        }
    }
}
```

### Usage in Render

```rust
impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .input_size(self.size)      // Apply height + padding
            .input_text_size(self.size) // Apply text size
            .child(self.label)
    }
}
```

### Usage by Users

```rust
Button::new("btn").label("Click").small()
Button::new("btn").label("Click").large()
Button::new("btn").label("Click").with_size(px(48.))
```
