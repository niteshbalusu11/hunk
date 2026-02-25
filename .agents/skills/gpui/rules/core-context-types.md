---
title: Know When to Use Different Context Types
impact: HIGH
tags: context, app, async-app, window
---

## Context Types

GPUI provides different context types for different situations.

### Context<T> - Entity-specific Operations

```rust
impl MyView {
    fn increment(&mut self, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();           // Trigger re-render
        cx.emit(CountChanged); // Emit typed event
    }
}
```

### App - Global Operations

```rust
fn setup_app(cx: &mut App) {
    // Register global state
    cx.set_global(MySettings::default());

    // Create entities
    let entity = cx.new(|cx| MyEntity::new(cx));

    // Observe globals
    cx.observe_global::<ThemeSettings>(|cx| {
        // Handle theme change
    }).detach();
}
```

### AsyncApp - Async Global Operations

```rust
async fn async_operation(mut cx: AsyncApp) -> Result<()> {
    // Access app state from async context
    let result = cx.update(|cx| {
        // Synchronous access to App
        cx.global::<MySettings>().clone()
    })?;

    // Async work
    let data = fetch_data().await;

    // Update state
    cx.update(|cx| {
        cx.global_mut::<MyState>().data = data;
    })?;

    Ok(())
}
```

### AsyncWindowContext - Async with Window Access

```rust
async fn window_async_operation(
    this: WeakEntity<Self>,
    mut cx: AsyncWindowContext,
) -> Result<()> {
    // Access window-specific state
    let bounds = this.update(&mut cx, |this, window, cx| {
        window.bounds()
    })??;

    // Async work
    let result = compute_layout(bounds).await;

    // Update with window context
    this.update_in(&mut cx, |this, window, cx| {
        this.layout = result;
        cx.notify();
    })?;

    Ok(())
}
```

## When to Use Each

| Context | Use Case |
|---------|----------|
| `Context<T>` | Inside entity methods, rendering |
| `App` | Global setup, non-entity operations |
| `AsyncApp` | Async operations without window |
| `AsyncWindowContext` | Async operations needing window |
