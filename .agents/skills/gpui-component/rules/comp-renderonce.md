---
title: Use RenderOnce with #[derive(IntoElement)]
impact: CRITICAL
tags: component, renderonce, stateless
---

## Stateless Component Pattern

gpui-component uses **Stateless RenderOnce** pattern for all components.

### Core Pattern

```rust
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    label: Option<SharedString>,
    disabled: bool,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // One-time render, consumes self
        self.base
            .id(self.id)
            .when(self.disabled, |this| this.opacity(0.5))
            .when_some(self.label, |this, label| this.child(label))
            .when_some(self.on_click, |this, handler| {
                this.on_click(move |e, w, cx| handler(e, w, cx))
            })
    }
}
```

### Key Points

1. **`#[derive(IntoElement)]`** - Auto-implements conversion to element
2. **`RenderOnce`** not `Render` - Component is consumed on render
3. **`Rc<dyn Fn>`** for callbacks - Allows multiple invocations
4. **`self` not `&mut self`** - Ownership transfer in render

### Why RenderOnce?

| Aspect | RenderOnce | Render (Entity) |
|--------|------------|-----------------|
| State | No internal state | Has internal state |
| Lifetime | Created and consumed each frame | Persists across frames |
| Memory | No Entity overhead | Requires Entity allocation |
| Use case | UI components | Views with state |

### Constructor Pattern

```rust
impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div().into_stateful(),
            style: StyleRefinement::default(),
            label: None,
            disabled: false,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}
```

### Usage

```rust
Button::new("submit")
    .label("Submit")
    .disabled(is_loading)
    .on_click(|_, window, cx| {
        // Handle click
    })
```
