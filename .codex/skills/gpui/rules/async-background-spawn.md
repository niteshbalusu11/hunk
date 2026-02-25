---
title: Use background_spawn for CPU-Intensive Work
impact: HIGH
tags: background, thread-pool, performance
---

## Background vs Foreground Tasks

GPUI has two execution contexts:
- **Foreground**: UI thread, all entity access and rendering
- **Background**: Thread pool, CPU-intensive work

### When to Use Background

**Incorrect (blocking UI thread):**

```rust
// BAD: CPU work blocks UI
fn process_file(&mut self, cx: &mut Context<Self>) {
    self.task = Some(cx.spawn(async move |this, mut cx| {
        // This runs on foreground, blocks UI!
        let processed = expensive_computation(data);

        this.update(&mut cx, |s, cx| {
            s.result = processed;
            cx.notify();
        })?;
        Ok(())
    }));
}
```

**Correct (use background_spawn):**

```rust
// GOOD: Heavy work on background thread
fn process_file(&mut self, cx: &mut Context<Self>) {
    let data = self.data.clone();

    self.task = Some(cx.spawn(async move |this, mut cx| {
        // Heavy work on background thread pool
        let processed = cx.background_spawn(async move {
            expensive_computation(data)
        }).await;

        // Update UI on foreground thread
        this.update(&mut cx, |state, cx| {
            state.result = processed;
            state.is_processing = false;
            cx.notify();
        })?;

        Ok(())
    }));
}
```

### Incremental Progress Updates

```rust
self.search_task = Some(cx.spawn(async move |project_search, cx| {
    let SearchResults { rx, .. } = search;

    // Receive results in batches
    let mut matches = pin!(rx.ready_chunks(1024));

    while let Some(results) = matches.next().await {
        // Process batch in background
        let processed = cx.background_executor()
            .spawn(async move {
                results.into_iter()
                    .map(process_result)
                    .collect::<Vec<_>>()
            })
            .await;

        // Yield to prevent UI starvation
        smol::future::yield_now().await;

        // Update UI with batch
        project_search.update(cx, |ps, cx| {
            ps.results.extend(processed);
            cx.notify();  // Incremental UI update
        })?;
    }

    Ok(())
}));
```

### Pattern Selection Guide

| Scenario | Use |
|----------|-----|
| Quick async I/O | `cx.spawn()` |
| CPU-intensive computation | `background_spawn()` |
| Long-running with progress | `background_spawn()` + `ready_chunks()` |
| Need window access | `cx.spawn_in(window, ...)` |
