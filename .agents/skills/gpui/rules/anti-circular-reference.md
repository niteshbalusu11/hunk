---
title: Avoid Entity Cycles, Use WeakEntity
impact: CRITICAL
tags: memory-leak, circular-reference, weak-entity
---

## Circular Reference Prevention

Entity cycles cause memory leaks because entities are reference-counted.

**Incorrect (memory leak):**

```rust
// BAD: Circular reference prevents deallocation
struct Parent {
    child: Entity<Child>,
}

struct Child {
    parent: Entity<Parent>,  // Creates cycle!
}

// Neither Parent nor Child can ever be deallocated
```

**Correct (use WeakEntity):**

```rust
// GOOD: Weak reference breaks cycle
struct Parent {
    child: Entity<Child>,  // Strong ref: parent owns child
}

struct Child {
    parent: WeakEntity<Parent>,  // Weak ref: child observes parent
}

impl Child {
    fn notify_parent(&self, cx: &mut Context<Self>) {
        // upgrade() returns Option<Entity<T>>
        if let Some(parent) = self.parent.upgrade() {
            parent.update(cx, |p, cx| {
                p.on_child_changed();
                cx.notify();
            });
        }
        // If parent is dropped, upgrade() returns None
    }
}
```

### Common Patterns

**Document-Editor Pattern:**
```rust
struct Document {
    // Weak refs to observers
    editors: Vec<WeakEntity<Editor>>,
}

struct Editor {
    // Strong ref to data source
    document: Entity<Document>,
}
```

**Parent-Child UI:**
```rust
struct TabBar {
    tabs: Vec<Entity<Tab>>,  // Strong: owns tabs
}

struct Tab {
    tab_bar: WeakEntity<TabBar>,  // Weak: reference to parent
}
```

**Subscription with Entity Reference:**
```rust
// GOOD: Use weak in subscriptions
let weak = entity.downgrade();

cx.subscribe(&other, move |this, other, event, cx| {
    // Only proceed if entity still exists
    if let Some(entity) = weak.upgrade() {
        entity.update(cx, |e, cx| {
            e.handle_event(event);
            cx.notify();
        });
    }
}).detach();
```

### Rule of Thumb

- **Parent → Child**: Use `Entity<T>` (strong)
- **Child → Parent**: Use `WeakEntity<T>` (weak)
- **Observer → Observed**: Usually weak
- **Data source**: Usually strong
