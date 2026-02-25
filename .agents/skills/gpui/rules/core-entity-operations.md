---
title: Use read/update/observe/subscribe Correctly
impact: CRITICAL
tags: entity, read, update, observe, subscribe
---

## Entity Operations

Understanding when to use each Entity operation is fundamental to GPUI development.

### Read vs Update

**Incorrect (mutating in read context):**

```rust
// BAD: Trying to mutate through read
let state = counter.read(cx);
state.count += 1; // Error: &T is immutable
```

**Correct (use update for mutation):**

```rust
// GOOD: Read for immutable access
let count = counter.read(cx).count;

// GOOD: Update for mutation
counter.update(cx, |state, cx| {
    state.count += 1;
    cx.notify();
});
```

### Observe vs Subscribe

**Use `observe` for any state change:**

```rust
// observe() triggers on ANY state change (when notify() is called)
cx.observe(&counter, |this, counter, cx| {
    let new_count = counter.read(cx).count;
    this.update_display(new_count);
}).detach();
```

**Use `subscribe` for typed events:**

```rust
// First, define event type
pub struct CounterEvent {
    pub delta: i32,
}
impl EventEmitter<CounterEvent> for Counter {}

// subscribe() triggers only on specific events
cx.subscribe(&counter, |this, counter, event: &CounterEvent, cx| {
    if event.delta > 10 {
        this.show_big_change_alert();
    }
}).detach();
```

## Data Flow

1. State changes happen through `entity.update(cx, ...)`
2. Call `cx.notify()` to signal the change
3. Observers registered with `cx.observe()` are called
4. Views re-render if they observe changed entities
5. Events emitted with `cx.emit()` trigger subscribers
