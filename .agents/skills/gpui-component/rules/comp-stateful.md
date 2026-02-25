---
title: Entity + RenderOnce for Stateful Components
impact: HIGH
tags: component, entity, stateful
---

## Stateful Component Pattern

For components that need internal state (like Input), use Entity for state + RenderOnce for rendering.

### Pattern Overview

```
┌──────────────────────────────────────────┐
│  Entity<InputState>  (persists)          │
│  - text: Rope                            │
│  - cursor: usize                         │
│  - focus_handle: FocusHandle             │
└──────────────────────────────────────────┘
                    │
                    │ reference
                    ▼
┌──────────────────────────────────────────┐
│  Input (RenderOnce)  (created each frame)│
│  - state: Entity<InputState>             │
│  - placeholder: Option<SharedString>     │
│  - style: StyleRefinement                │
└──────────────────────────────────────────┘
```

### State Definition

```rust
pub struct InputState {
    text: Rope,
    cursor: usize,
    selection: Option<Range<usize>>,
    focus_handle: FocusHandle,
    disabled: bool,
}

impl InputState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            text: Rope::default(),
            cursor: 0,
            selection: None,
            focus_handle: cx.focus_handle(),
            disabled: false,
        }
    }

    pub fn value(&self) -> String {
        self.text.to_string()
    }

    pub fn set_value(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        self.text = Rope::from(value.into());
        cx.notify();
    }

    // Action handlers
    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.cursor > 0 {
            self.text.delete(self.cursor - 1..self.cursor);
            self.cursor -= 1;
            cx.notify();
        }
    }
}
```

### Rendering Element

```rust
#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    placeholder: Option<SharedString>,
    style: StyleRefinement,
    cleanable: bool,
}

impl Input {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            placeholder: None,
            style: StyleRefinement::default(),
            cleanable: false,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let focused = state.focus_handle.is_focused(window);
        let value = state.value();

        div()
            .track_focus(&state.focus_handle)
            .on_action(window.listener_for(&self.state, InputState::backspace))
            .when(value.is_empty(), |this| {
                this.when_some(self.placeholder.clone(), |this, ph| {
                    this.child(div().text_color(cx.theme().muted).child(ph))
                })
            })
            .when(!value.is_empty(), |this| {
                this.child(value)
            })
            .when(self.cleanable && !value.is_empty(), |this| {
                this.child(
                    Icon::new(IconName::X)
                        .on_click({
                            let state = self.state.clone();
                            move |_, window, cx| {
                                state.update(cx, |s, cx| s.set_value("", cx));
                            }
                        })
                )
            })
    }
}
```

### Usage in Parent View

```rust
struct MyForm {
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
}

impl MyForm {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            name_input: cx.new(|cx| InputState::new(window, cx)),
            email_input: cx.new(|cx| InputState::new(window, cx)),
        }
    }
}

impl Render for MyForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .child(Input::new(&self.name_input).placeholder("Name"))
            .child(Input::new(&self.email_input).placeholder("Email"))
            .child(
                Button::new("submit")
                    .label("Submit")
                    .on_click({
                        let name = self.name_input.clone();
                        let email = self.email_input.clone();
                        move |_, window, cx| {
                            let name_value = name.read(cx).value();
                            let email_value = email.read(cx).value();
                            // Submit form
                        }
                    })
            )
    }
}
```
