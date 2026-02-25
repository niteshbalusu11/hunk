---
title: Store or Detach Tasks to Prevent Cancellation
impact: CRITICAL
tags: task, async, lifecycle, cancellation
---

## Task Lifecycle Management

Tasks in GPUI are cancelled when dropped. You must explicitly manage their lifecycle.

**Incorrect (task cancelled immediately):**

```rust
// BAD: Task is dropped immediately and cancelled
fn fetch_data(&mut self, cx: &mut Context<Self>) {
    cx.spawn(async move |this, mut cx| {
        let data = fetch().await;
        this.update(&mut cx, |s, cx| {
            s.data = data;
            cx.notify();
        })?;
        Ok(())
    }); // Task dropped here!
}
```

**Correct (store task for control):**

```rust
// GOOD: Store task to control lifetime
struct MyView {
    fetch_task: Option<Task<()>>,
}

fn fetch_data(&mut self, cx: &mut Context<Self>) {
    self.fetch_task = Some(cx.spawn(async move |this, mut cx| {
        let data = fetch().await;
        this.update(&mut cx, |s, cx| {
            s.data = data;
            cx.notify();
        })?;
        Ok(())
    }));
}
```

**Or detach if you don't need to cancel:**

```rust
// GOOD: Detach for fire-and-forget
cx.spawn(async move |this, mut cx| {
    let data = fetch().await;
    this.update(&mut cx, |s, cx| {
        s.data = data;
        cx.notify();
    })?;
    Ok(())
}).detach();

// With error logging
cx.spawn(async move |this, mut cx| {
    // ...
}).detach_and_log_err(cx);
```

## Task Storage Patterns

```rust
pub struct Editor {
    // Simple tasks bound to entity lifetime
    refresh_task: Task<()>,

    // Cancellable tasks use Option
    search_task: Option<Task<Result<()>>>,

    // Shared tasks for multiple awaiters
    load_task: Option<Shared<Task<()>>>,

    // Multiple parallel tasks
    completion_tasks: Vec<(CompletionId, Task<()>)>,
}
```

## When to Use Each

| Pattern | Use Case |
|---------|----------|
| `.detach()` | Fire and forget, no cancellation needed |
| `.detach_and_log_err(cx)` | Fire and forget, log errors |
| `Option<Task<>>` | Need to cancel or replace |
| `Task<()>` field | Bound to entity lifetime |
| `Shared<Task<>>` | Multiple awaiters need result |
