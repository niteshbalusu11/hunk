---
title: Implement Debounce with Timer and Task Replacement
impact: HIGH
tags: debounce, throttle, timer, async
---

## Debounce Pattern

Debounce prevents rapid repeated calls by waiting for a pause in activity.

### Simple Debounce

```rust
struct Search {
    search_task: Option<Task<()>>,
}

impl Search {
    fn on_query_changed(&mut self, query: String, cx: &mut Context<Self>) {
        // Cancel previous search by replacing task
        self.search_task = Some(cx.spawn(async move |this, mut cx| {
            // Wait for typing to pause
            cx.background_executor()
                .timer(Duration::from_millis(150))
                .await;

            let results = search(&query).await;

            this.update(&mut cx, |search, cx| {
                search.results = results;
                cx.notify();
            })?;

            Ok(())
        }));
    }
}
```

### Debounce with Validation

```rust
self.code_actions_task = Some(cx.spawn(async move |this, cx| {
    // Debounce delay
    cx.background_executor()
        .timer(Duration::from_millis(200))
        .await;

    // Validate state is still relevant after delay
    let data = this
        .update(cx, |this, cx| {
            // Check if selection changed during delay
            if this.selections.newest_anchor() != expected_selection {
                return None;  // State changed, abort
            }
            Some(this.get_relevant_data())
        })?
        .context("State changed during debounce")?;

    // Execute actual work
    let results = fetch_code_actions(data).await;

    // Update UI
    this.update(cx, |this, cx| {
        this.code_actions = results;
        cx.notify();
    })?;

    Ok(())
}));
```

### Throttle Pattern

Throttle limits execution rate instead of delaying:

```rust
fn save_bounds(&mut self, cx: &mut Context<Self>) {
    // Only save at most every 100ms
    self.bounds_save_task = Some(cx.spawn(async move |this, cx| {
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;

        this.update_in(cx, |this, window, cx| {
            if let Some(display) = window.display(cx) {
                this.persist_bounds(display);
            }
        })?;

        Ok(())
    }));
}
```

### DebouncedDelay Utility

For complex debounce with cancellation:

```rust
pub struct DebouncedDelay<E: 'static> {
    task: Option<Task<()>>,
    cancel_channel: Option<oneshot::Sender<()>>,
}

impl<E: 'static> DebouncedDelay<E> {
    pub fn fire_new<F>(&mut self, delay: Duration, cx: &mut Context<E>, func: F)
    where
        F: 'static + Send + FnOnce(&mut E, &mut Context<E>) -> Task<()>,
    {
        // Cancel previous
        if let Some(channel) = self.cancel_channel.take() {
            _ = channel.send(());
        }

        let (sender, mut receiver) = oneshot::channel::<()>();
        self.cancel_channel = Some(sender);

        self.task = Some(cx.spawn(async move |entity, cx| {
            let mut timer = cx.background_executor().timer(delay).fuse();

            futures::select_biased! {
                _ = receiver => return,  // Cancelled
                _ = timer => {}          // Timeout elapsed
            }

            if let Ok(task) = entity.update(cx, |e, cx| (func)(e, cx)) {
                task.await;
            }
        }));
    }
}
```
