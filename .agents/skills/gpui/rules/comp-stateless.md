---
title: Prefer RenderOnce with #[derive(IntoElement)]
impact: MEDIUM
tags: component, renderonce, stateless
---

## Stateless Component Pattern

For reusable UI components, prefer the stateless RenderOnce pattern.

### Component Template

```rust
#[derive(IntoElement)]
pub struct MyComponent {
    // 1. Identifier
    id: ElementId,

    // 2. Style override
    style: StyleRefinement,

    // 3. Content
    label: Option<SharedString>,
    children: Vec<AnyElement>,

    // 4. State configuration
    disabled: bool,
    selected: bool,
    size: Size,

    // 5. Callbacks (use Rc for multiple calls)
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl MyComponent {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            label: None,
            children: Vec::new(),
            disabled: false,
            selected: false,
            size: Size::default(),
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

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MyComponent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .when(self.disabled, |this| {
                this.opacity(0.5).cursor_not_allowed()
            })
            .when_some(self.label, |this, label| {
                this.child(label)
            })
            .children(self.children)
            .when_some(self.on_click, |this, handler| {
                this.on_click(move |e, w, cx| handler(e, w, cx))
            })
    }
}
```

### Implementing Styled

```rust
impl Styled for MyComponent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

// Now supports: .bg(), .p_4(), .rounded_lg(), etc.
```

### Implementing ParentElement

```rust
impl ParentElement for MyComponent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

// Now supports: .child(), .children()
```

### When to Use Entity Instead

Use `Entity<T>` + `Render` when:
- Component has internal mutable state
- State persists across re-renders
- Need to respond to external events

```rust
// Input with internal state
pub struct InputState {
    text: Rope,
    cursor: usize,
    focus_handle: FocusHandle,
}

// Rendering wrapper
#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    placeholder: Option<SharedString>,
}
```
