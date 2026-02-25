---
title: Never Forget cx.notify() After State Changes
impact: CRITICAL
tags: notify, re-render, state
---

## Notify After State Changes

Views only re-render when `cx.notify()` is called. Missing this causes UI to not update.

**Incorrect (UI won't update):**

```rust
// BAD: View won't re-render
fn increment(&mut self, cx: &mut Context<Self>) {
    self.count += 1;
    // Missing cx.notify()!
}

fn set_items(&mut self, items: Vec<Item>, cx: &mut Context<Self>) {
    self.items = items;
    // UI still shows old items!
}
```

**Correct (trigger re-render):**

```rust
// GOOD: UI updates
fn increment(&mut self, cx: &mut Context<Self>) {
    self.count += 1;
    cx.notify();
}

fn set_items(&mut self, items: Vec<Item>, cx: &mut Context<Self>) {
    self.items = items;
    cx.notify();
}
```

### When to Notify

| Scenario | Notify? |
|----------|---------|
| Changed visible state | Yes |
| Changed internal-only data | No |
| Multiple changes | Once at end |
| In async callback | Yes, inside update |

### Batch Updates

```rust
// GOOD: Single notify for multiple changes
fn reset(&mut self, cx: &mut Context<Self>) {
    self.count = 0;
    self.history.clear();
    self.modified = false;
    cx.notify();  // One notify at the end
}
```

### Async Pattern

```rust
cx.spawn(async move |this, mut cx| {
    let data = fetch_data().await;

    this.update(&mut cx, |state, cx| {
        state.data = data;
        cx.notify();  // Inside update closure
    })?;

    Ok(())
}).detach();
```

### Observer Chain

When entity A observes entity B, notify cascades:

```rust
// In entity B
self.value = new_value;
cx.notify();  // Triggers observers

// Observer in entity A
cx.observe(&entity_b, |this, b, cx| {
    // Called when B notifies
    this.cached_value = b.read(cx).value;
    cx.notify();  // A also re-renders
}).detach();
```
