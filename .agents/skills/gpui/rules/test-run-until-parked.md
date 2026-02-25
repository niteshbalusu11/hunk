---
title: Always Call run_until_parked After Async Operations
impact: CRITICAL
tags: testing, async, run-until-parked
---

## run_until_parked Pattern

`run_until_parked()` waits for all pending tasks to complete. This is essential for reliable tests.

**Incorrect (race condition):**

```rust
// BAD: Task may not have completed
#[gpui::test]
async fn test_async(cx: &mut TestAppContext) {
    entity.update(cx, |e, cx| {
        e.start_async_operation(cx);  // Spawns task
    });

    // Task hasn't finished!
    assert!(entity.read(cx).completed);  // May fail!
}
```

**Correct (wait for tasks):**

```rust
// GOOD: Wait for all tasks
#[gpui::test]
async fn test_async(cx: &mut TestAppContext) {
    entity.update(cx, |e, cx| {
        e.start_async_operation(cx);
    });

    cx.run_until_parked();  // Wait for all tasks

    assert!(entity.read(cx).completed);  // Reliable
}
```

### When to Use

```rust
// After spawning tasks
cx.spawn(async { ... }).detach();
cx.run_until_parked();

// After UI updates that trigger async work
view.update(&mut cx, |v, w, cx| v.search("query", w, cx));
cx.run_until_parked();

// After time advancement
cx.executor().advance_clock(Duration::from_secs(1));
cx.run_until_parked();

// After simulating events
cx.simulate_click(point, modifiers);
cx.run_until_parked();
```

### Time Simulation

```rust
// Advance simulated time
cx.executor().advance_clock(Duration::from_secs(2));
cx.run_until_parked();

// Jump to next delayed task
cx.executor().advance_clock_to_next_delayed();
cx.run_until_parked();

// Test timeout behavior
cx.executor().advance_clock(TIMEOUT_DURATION);
cx.run_until_parked();
```

### Use GPUI Timers

```rust
// GOOD: Uses TestDispatcher, works with run_until_parked
cx.background_executor().timer(Duration::from_millis(100)).await;

// BAD: May not work with run_until_parked
smol::Timer::after(Duration::from_millis(100)).await;  // Avoid!
```
