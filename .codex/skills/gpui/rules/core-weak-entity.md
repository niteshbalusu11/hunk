---
title: Use WeakEntity to Break Circular References
impact: CRITICAL
tags: weak-entity, memory, circular-reference
---

## WeakEntity for Cycle Prevention

Use `WeakEntity<T>` to avoid memory leaks from circular references between entities.

**Incorrect (circular reference causes memory leak):**

```rust
// BAD: Circular reference prevents deallocation
struct Parent {
    child: Entity<Child>,
}

struct Child {
    parent: Entity<Parent>, // Creates cycle!
}
```

**Correct (weak reference breaks cycle):**

```rust
// GOOD: Weak reference breaks cycle
struct Child {
    parent: WeakEntity<Parent>,
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
    }
}
```

### Use in Subscriptions

```rust
// GOOD: Use weak in subscriptions to avoid cycles
let weak = entity.downgrade();

cx.subscribe(&other, move |this, other, event, cx| {
    if let Some(entity) = weak.upgrade() {
        entity.update(cx, |e, cx| {
            e.handle_event(event);
            cx.notify();
        });
    }
}).detach();
```

### Common Pattern: Back-references

```rust
struct Document {
    editors: Vec<WeakEntity<Editor>>,  // Weak refs to observers
}

struct Editor {
    document: Entity<Document>,  // Strong ref to data source
}
```

This pattern ensures:
- Document doesn't keep editors alive
- Editors keep document alive as long as needed
- No memory leaks when editors are closed
