---
title: Never Drop Task Without Storing or Detaching
impact: CRITICAL
tags: task, async, cancellation
---

## Task Lifecycle

Dropping a Task cancels it immediately. This is a common source of bugs.

**Incorrect (task cancelled immediately):**

```rust
// BAD: Task is dropped and cancelled
fn fetch_data(&mut self, cx: &mut Context<Self>) {
    cx.spawn(async move |this, mut cx| {
        let data = fetch().await;  // Never completes!
        this.update(&mut cx, |s, cx| {
            s.data = data;
            cx.notify();
        })?;
        Ok(())
    }); // Dropped here, cancelled!
}
```

**Correct (store or detach):**

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

// GOOD: Detach for fire-and-forget
fn log_event(&mut self, cx: &mut Context<Self>) {
    cx.spawn(async move |_, _| {
        send_analytics_event().await;
        Ok(())
    }).detach();  // Explicitly fire-and-forget
}
```

### Detach Patterns

```rust
// Fire and forget
task.detach();

// Fire and log errors
task.detach_and_log_err(cx);

// Fire and show errors to user
task.detach_and_notify_err(window, cx);
```

### Task Storage Patterns

```rust
struct Editor {
    // Required task, bound to entity lifetime
    refresh_task: Task<()>,

    // Cancellable task
    search_task: Option<Task<Result<()>>>,

    // Multiple tasks of same type
    pending_requests: Vec<Task<()>>,

    // Task with associated data
    debounced_task: Option<(Range<Anchor>, Task<()>)>,
}
```

### Cancelling Tasks

```rust
// Cancel by replacing
self.search_task = Some(new_task);  // Old task dropped/cancelled

// Cancel explicitly
self.search_task.take();  // Task dropped/cancelled

// Cancel all
self.pending_requests.clear();
```
