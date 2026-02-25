---
title: Understand GPUI's Single Ownership Model
impact: CRITICAL
tags: ownership, entity, app, architecture
---

## GPUI Ownership Model

GPUI uses a single ownership model where all application state (Entity) is owned by `App`. Entity handles act as smart pointers (like `Rc`) but cannot directly access state - you must go through a context.

**Incorrect (trying to access state directly):**

```rust
// BAD: Cannot access entity state directly
let counter: Entity<Counter> = cx.new(|_| Counter { count: 0 });
let count = counter.count; // Error: Entity doesn't expose fields
```

**Correct (access through context):**

```rust
// GOOD: Access state through read/update
let counter: Entity<Counter> = cx.new(|_| Counter { count: 0 });

// Immutable access
let count = counter.read(cx).count;

// Mutable access with notification
counter.update(cx, |counter, cx| {
    counter.count += 1;
    cx.notify(); // Signal that state changed
});
```

## Architecture

```
┌─────────────────────────────────────┐
│      Application (App)              │
│   (Single owner of all Entities)    │
└─────────────────────────────────────┘
         │
    ┌────┼────┐
    │    │    │
Entity<A> Entity<B> Global<C>
    │    │    │
 Context<A> Context<B> via App
```

## Entity Operations

| Operation | Returns | Use Case |
|-----------|---------|----------|
| `entity.entity_id()` | `EntityId` | Unique identifier |
| `entity.downgrade()` | `WeakEntity<T>` | Break reference cycles |
| `entity.read(cx)` | `&T` | Immutable access |
| `entity.update(cx, \|t, cx\| ...)` | Closure return | Mutable access |
