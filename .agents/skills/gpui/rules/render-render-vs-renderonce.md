---
title: Choose Render for Stateful Views, RenderOnce for Components
impact: CRITICAL
tags: render, renderonce, component, view
---

## Two Rendering Traits

GPUI has two rendering traits for different use cases.

### Render - Stateful Entity Views

Use `Render` for `Entity<T>` that hold state and need to re-render.

```rust
struct Editor {
    content: String,
    cursor: usize,
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_toolbar(window, cx))
            .child(self.render_content(window, cx))
    }
}
```

### RenderOnce - Stateless Components

Use `RenderOnce` for reusable UI components without internal state.

```rust
#[derive(IntoElement)]
pub struct Avatar {
    image: Img,
    size: Option<AbsoluteLength>,
}

impl Avatar {
    pub fn new(src: impl Into<ImageSource>) -> Self {
        Self {
            image: img(src),
            size: None,
        }
    }

    pub fn size(mut self, size: impl Into<AbsoluteLength>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl RenderOnce for Avatar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let size = self.size.unwrap_or(px(24.));
        div()
            .size(size)
            .rounded_full()
            .overflow_hidden()
            .child(self.image.size(size))
    }
}
```

## Key Differences

| Aspect | Render | RenderOnce |
|--------|--------|------------|
| Self | `&mut self` | `self` (consumes) |
| Context | `Context<T>` | `App` |
| Use case | Stateful views | Reusable components |
| Derive | N/A | `#[derive(IntoElement)]` |
| Re-render | Via cx.notify() | Rebuilt each time |

## Best Practice

- **Views with state** → `Render` trait
- **Reusable UI components** → `RenderOnce` + `#[derive(IntoElement)]`
- **Components needing Entity state** → Pass `Entity<T>` to RenderOnce component
