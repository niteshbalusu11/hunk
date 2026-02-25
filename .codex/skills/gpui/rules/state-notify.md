---
title: Always Call cx.notify() After State Changes
impact: CRITICAL
tags: notify, state, re-render
---

## Notify After State Changes

Views only re-render when `cx.notify()` is called. Forgetting to notify is a common bug.

**Incorrect (view won't re-render):**

```rust
// BAD: View won't re-render
fn increment(&mut self, cx: &mut Context<Self>) {
    self.count += 1;
    // Missing cx.notify()!
}
```

**Correct (trigger re-render):**

```rust
// GOOD: View will re-render
fn increment(&mut self, cx: &mut Context<Self>) {
    self.count += 1;
    cx.notify();
}
```

### When to Notify

| Scenario | Action |
|----------|--------|
| Direct field mutation | Call `cx.notify()` |
| Multiple mutations | Call `cx.notify()` once at end |
| No visible change | Skip `cx.notify()` |
| In async callback | Call `cx.notify()` in update closure |

### Async Pattern

```rust
cx.spawn(async move |this, mut cx| {
    let data = fetch_data().await;

    this.update(&mut cx, |state, cx| {
        state.data = data;
        cx.notify();  // Notify inside update closure
    })?;

    Ok(())
}).detach();
```

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

### Observer Notification

When you update an entity, all observers are notified:

```rust
// Observer setup
cx.observe(&counter, |this, counter, cx| {
    // Called when counter.notify() is called
    let count = counter.read(cx).count;
    this.update_display(count);
}).detach();

// Trigger observer
counter.update(cx, |c, cx| {
    c.count += 1;
    cx.notify();  // This triggers the observer above
});
```
