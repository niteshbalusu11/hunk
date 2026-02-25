---
title: Use WeakEntity for Safe Cross-Await Access
impact: HIGH
tags: weak-entity, async, safety
---

## Safe Async Access with WeakEntity

In async code, entities might be dropped while awaiting. Use WeakEntity for safety.

**How cx.spawn Works:**

```rust
// cx.spawn provides a WeakEntity automatically
self.task = Some(cx.spawn(async move |this, mut cx| {
    // `this` is WeakEntity<Self>

    // .update() returns Result - fails if Entity dropped
    this.update(&mut cx, |state, cx| {
        state.value = 42;
        cx.notify();
    })?;  // Handle the Result!

    Ok(())
}));
```

### Safe State Access Pattern

```rust
self.task = Some(cx.spawn(async move |this, cx| {
    // Step 1: Extract data before async work
    let data = this
        .update(cx, |this, cx| {
            this.get_data_for_processing()
        })?
        .context("Failed to get data")?;

    // Step 2: Async work (entity might be dropped here)
    let result = fetch_data(data).await;

    // Step 3: Update state (handles dropped entity)
    this.update(cx, |this, cx| {
        this.result = result;
        cx.notify();
    })?;

    Ok(())
}));
```

### Using update_in for Window Access

```rust
self.task = Some(cx.spawn_in(window, async move |this, cx| {
    // update_in provides window access
    let (providers, tasks) = this.update_in(cx, |this, window, cx| {
        let providers = this.providers.clone();
        let tasks = providers
            .iter()
            .map(|p| p.fetch(window, cx))
            .collect::<Vec<_>>();
        (providers, tasks)
    })?;

    // Wait for tasks
    let results = future::join_all(tasks).await;

    // Update with results
    this.update(cx, |this, cx| {
        this.results = results;
        cx.notify();
    })?;

    Ok(())
}));
```

### Clone Data Before Async

For data that shouldn't change during async work:

```rust
let data = self.data.clone();  // Clone before spawn

self.task = Some(cx.spawn(async move |this, cx| {
    // data is now an independent copy
    let result = process_data(data).await;

    this.update(cx, |this, cx| {
        this.result = result;
        cx.notify();
    })?;

    Ok(())
}));
```

### Error Handling in Async

```rust
// Propagate errors
this.update(cx, |this, cx| {
    this.get_data()
})?.context("Failed to get data")?;

// Log and continue
if let Some(data) = this.update(cx, |this, cx| {
    this.get_optional_data()
}).log_err() {
    process_data(data);
}
```
