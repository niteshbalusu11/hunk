---
title: Never Silently Discard Errors
impact: CRITICAL
tags: error, result, logging
---

## Error Handling

Silent error discarding hides bugs and makes debugging difficult.

**Incorrect (silent discard):**

```rust
// BAD: Error is silently discarded
let _ = client.request(...).await;
let _ = entity.update(&mut cx, |e, cx| { ... });
```

**Correct (handle errors properly):**

```rust
// GOOD: Propagate error
client.request(...).await?;

// GOOD: Log error
client.request(...).await.log_err();

// GOOD: Handle explicitly
match client.request(...).await {
    Ok(response) => { /* handle success */ }
    Err(e) => {
        log::error!("Request failed: {}", e);
    }
}
```

### ResultExt Trait

```rust
pub trait ResultExt<E> {
    type Ok;
    fn log_err(self) -> Option<Self::Ok>;
    fn warn_on_err(self) -> Option<Self::Ok>;
    fn debug_assert_ok(self, reason: &str) -> Self;
}

// Usage
if let Some(data) = fetch_data().await.log_err() {
    process_data(data);
}
```

### Async Error Patterns

```rust
// Propagate and log
cx.spawn(async move |this, mut cx| {
    let result = this.update(&mut cx, |s, cx| s.value = 42)?;
    Ok(())
}).detach_and_log_err(cx);

// Show error to user
save_task.detach_and_notify_err(window, cx);
```

### When Discard is Acceptable

```rust
// OK: Intentionally ignoring a cancel signal
_ = cancel_sender.send(());

// OK: Best-effort cleanup
_ = fs::remove_file(temp_path);

// OK: Optional notification
_ = optional_channel.try_send(update);
```

Use `_` only when you explicitly intend to ignore the result and understand the consequences.
