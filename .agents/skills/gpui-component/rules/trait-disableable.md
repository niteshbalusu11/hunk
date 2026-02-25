---
title: Implement Disableable for Disabled State
impact: HIGH
tags: trait, disableable, state
---

## Disableable Trait

Implement `Disableable` for components that can be disabled.

### Trait Definition

```rust
pub trait Disableable {
    fn disabled(self, disabled: bool) -> Self;
}
```

### Implementation

```rust
#[derive(IntoElement)]
pub struct Button {
    disabled: bool,
    // ...
}

impl Disableable for Button {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}
```

### Applying Disabled State in Render

```rust
impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            // Disabled styling
            .when(self.disabled, |this| {
                this
                    .opacity(0.5)
                    .cursor_not_allowed()
            })
            // Disable focus when disabled
            .when(!self.disabled, |this| {
                this.track_focus(&self.focus_handle)
            })
            // Disable click when disabled
            .when_some(self.on_click, |this, handler| {
                if self.disabled {
                    this
                } else {
                    this.on_click(move |e, w, cx| handler(e, w, cx))
                }
            })
    }
}
```

### Selectable Trait

Similar pattern for selection state:

```rust
pub trait Selectable: Sized {
    fn selected(self, selected: bool) -> Self;
    fn is_selected(&self) -> bool;
}

impl Selectable for Button {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}
```

### Collapsible Trait

For expandable components:

```rust
pub trait Collapsible {
    fn collapsed(self, collapsed: bool) -> Self;
    fn is_collapsed(&self) -> bool;
}
```

### Usage

```rust
Button::new("submit")
    .label("Submit")
    .disabled(is_loading || !is_valid)

Checkbox::new("agree")
    .label("I agree")
    .selected(agreed)
    .disabled(submitted)
```
