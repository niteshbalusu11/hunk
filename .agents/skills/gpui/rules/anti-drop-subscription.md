---
title: Always Detach or Store Subscriptions
impact: CRITICAL
tags: subscription, observe, subscribe
---

## Subscription Lifecycle

Like Tasks, Subscriptions are cancelled when dropped.

**Incorrect (subscription dropped immediately):**

```rust
// BAD: Subscription dropped, observer never called
fn new(cx: &mut Context<Self>) -> Self {
    cx.observe(&other, |this, other, cx| {
        // This never runs!
    }); // Subscription dropped here

    Self { ... }
}
```

**Correct (detach or store):**

```rust
// GOOD: Detach subscription
fn new(cx: &mut Context<Self>) -> Self {
    cx.observe(&other, |this, other, cx| {
        this.on_other_changed(other, cx);
    }).detach();  // Lives until entity is dropped

    Self { ... }
}

// GOOD: Store subscriptions
struct MyView {
    _subscriptions: Vec<Subscription>,
}

fn new(cx: &mut Context<Self>) -> Self {
    let subscriptions = vec![
        cx.observe(&other, Self::on_other_changed),
        cx.subscribe(&buffer, Self::on_buffer_event),
        cx.observe_global::<Theme>(Self::on_theme_changed),
    ];

    Self {
        _subscriptions: subscriptions,
    }
}
```

### Subscription Methods

```rust
// Observe any state change
cx.observe(&entity, |this, entity, cx| {
    let value = entity.read(cx).value;
    this.update_display(value);
}).detach();

// Subscribe to typed events
cx.subscribe(&entity, |this, entity, event: &MyEvent, cx| {
    this.handle_event(event);
}).detach();

// Observe global state
cx.observe_global::<Theme>(|this, cx| {
    this.refresh_theme();
    cx.notify();
}).detach();

// Observe window activation
cx.observe_window_activation(|this, window, cx| {
    if window.is_active() {
        this.on_activated(cx);
    }
}).detach();
```

### Why Store vs Detach?

```rust
// Detach: Lives until entity is dropped
// Use when subscription should last for entity lifetime
cx.observe(&other, ...).detach();

// Store: Can unsubscribe manually
// Use when you need to stop observing
let sub = cx.observe(&other, ...);
self.subscriptions.push(sub);
// Later:
self.subscriptions.clear();  // Unsubscribes
```
