---
title: Standard Component Structure Template
impact: CRITICAL
tags: component, structure, template
---

## Component Structure

Follow this standard structure for all gpui-component components.

### Complete Template

```rust
#[derive(IntoElement)]
pub struct MyComponent {
    // 1. Identifier (required for stateful interactions)
    id: ElementId,

    // 2. Base element (for event delegation)
    base: Div,  // or Stateful<Div> if needs hover/active states

    // 3. Style override (allows .bg(), .p_4(), etc.)
    style: StyleRefinement,

    // 4. Content
    label: Option<SharedString>,
    icon: Option<IconName>,
    children: Vec<AnyElement>,

    // 5. State configuration
    disabled: bool,
    selected: bool,
    loading: bool,
    size: Size,
    variant: ComponentVariant,

    // 6. Callbacks (use Rc for multi-call)
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    on_change: Option<Rc<dyn Fn(bool, &mut Window, &mut App)>>,

    // 7. Focus/Tab (for keyboard navigation)
    tab_index: isize,
    tab_stop: bool,
}
```

### Constructor

```rust
impl MyComponent {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            style: StyleRefinement::default(),
            label: None,
            icon: None,
            children: Vec::new(),
            disabled: false,
            selected: false,
            loading: false,
            size: Size::default(),
            variant: ComponentVariant::default(),
            on_click: None,
            on_change: None,
            tab_index: 0,
            tab_stop: true,
        }
    }
}
```

### Builder Methods

```rust
impl MyComponent {
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
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

### RenderOnce Implementation

```rust
impl RenderOnce for MyComponent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();

        self.base
            .id(self.id.clone())
            // Apply stored styles
            .style(self.style)
            // Size-based styling
            .input_size(self.size)
            .input_text_size(self.size)
            // State-based styling
            .when(self.disabled, |this| {
                this.opacity(0.5).cursor_not_allowed()
            })
            .when(self.selected, |this| {
                this.bg(colors.primary)
            })
            // Content
            .when_some(self.icon, |this, icon| {
                this.child(Icon::new(icon))
            })
            .when_some(self.label, |this, label| {
                this.child(label)
            })
            .children(self.children)
            // Events
            .when_some(self.on_click, |this, handler| {
                this.on_click(move |e, w, cx| handler(e, w, cx))
            })
    }
}
```
