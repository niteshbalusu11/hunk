---
title: Entity-based Input State Management
impact: MEDIUM
tags: form, input, state
---

## Input State Pattern

Form inputs use Entity for state management, RenderOnce for rendering.

### InputState Definition

```rust
pub struct InputState {
    text: Rope,
    cursor: usize,
    selection: Option<Range<usize>>,
    focus_handle: FocusHandle,
    disabled: bool,
    placeholder: Option<SharedString>,
}

impl InputState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            text: Rope::default(),
            cursor: 0,
            selection: None,
            focus_handle: cx.focus_handle(),
            disabled: false,
            placeholder: None,
        }
    }

    // Getters
    pub fn value(&self) -> String {
        self.text.to_string()
    }

    pub fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }

    // Setters
    pub fn set_value(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        let value = value.into();
        self.text = Rope::from(value.as_str());
        self.cursor = value.len();
        cx.notify();
    }

    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        cx.notify();
    }

    // Actions
    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(sel) = self.selection.take() {
            self.text.delete(sel.clone());
            self.cursor = sel.start;
        } else if self.cursor > 0 {
            self.text.delete(self.cursor - 1..self.cursor);
            self.cursor -= 1;
        }
        cx.notify();
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        let len = self.text.len();
        if self.cursor < len {
            self.text.delete(self.cursor..self.cursor + 1);
        }
        cx.notify();
    }
}
```

### Input Render Element

```rust
#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    placeholder: Option<SharedString>,
    cleanable: bool,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
}

impl Input {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            placeholder: None,
            cleanable: false,
            prefix: None,
            suffix: None,
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
```

### Usage in Parent Component

```rust
struct LoginForm {
    username: Entity<InputState>,
    password: Entity<InputState>,
}

impl LoginForm {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            username: cx.new(|cx| InputState::new(window, cx)),
            password: cx.new(|cx| InputState::new(window, cx)),
        }
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let username = self.username.read(cx).value();
        let password = self.password.read(cx).value();
        // Submit login
    }
}

impl Render for LoginForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_4()
            .child(Input::new(&self.username).placeholder("Username"))
            .child(Input::new(&self.password).placeholder("Password"))
            .child(
                Button::new("submit")
                    .label("Login")
                    .primary()
                    .on_click(cx.listener(Self::submit))
            )
    }
}
```

### Focus Management

```rust
// Focus input programmatically
self.username.read(cx).focus_handle(cx).focus(window, cx);

// Check if focused
let focused = self.username.read(cx).focus_handle(cx).is_focused(window);
```
