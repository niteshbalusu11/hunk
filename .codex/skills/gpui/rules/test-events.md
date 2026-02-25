---
title: Testing Events and Notifications
impact: HIGH
tags: testing, events, notifications
---

## Event Testing Patterns

GPUI provides utilities for observing and waiting for events in tests.

### Observing Notifications

```rust
#[gpui::test]
fn test_notifications(cx: &mut TestAppContext) {
    let entity = cx.new(|_| Counter { count: 0 });

    // Create notification stream
    let mut notifications = cx.notifications(&entity);

    // Trigger notification
    entity.update(cx, |c, cx| {
        c.count += 1;
        cx.notify();
    });

    // Verify notification was sent
    assert!(notifications.next().now_or_never().is_some());
}
```

### Subscribing to Events

```rust
#[gpui::test]
fn test_events(cx: &mut TestAppContext) {
    let entity = cx.new(|_| MyEntity::new());

    // Subscribe to events
    let mut events = cx.events::<MyEvent, _>(&entity);

    // Trigger event
    entity.update(cx, |e, cx| {
        cx.emit(MyEvent::Changed);
    });

    // Check event
    let event = events.next().now_or_never().unwrap().unwrap();
    assert!(matches!(event, MyEvent::Changed));
}
```

### Waiting for Conditions

```rust
#[gpui::test]
async fn test_condition(cx: &mut TestAppContext) {
    let entity = cx.new(|_| Counter { count: 0 });

    // Start async operation
    entity.update(cx, |c, cx| {
        cx.spawn(async move |this, mut cx| {
            this.update(&mut cx, |c, cx| {
                c.count = 10;
                cx.notify();
            })
        }).detach();
    });

    // Wait for condition (with 3s timeout)
    cx.condition(&entity, |c, _| c.count >= 10).await;

    assert_eq!(entity.read(cx).count, 10);
}
```

### Capturing Single Event

```rust
#[gpui::test]
async fn test_single_event(cx: &mut TestAppContext) {
    let entity = cx.new(|_| MyEntity::new());

    // Set up event capture
    let event_future = entity.next_event::<MyEvent>(cx);

    // Trigger event
    entity.update(cx, |e, cx| {
        cx.emit(MyEvent::Completed { result: 42 });
    });

    // Get the event
    let event = event_future.await;
    assert!(matches!(event, MyEvent::Completed { result: 42 }));
}
```

### Collecting Multiple Events

```rust
#[gpui::test]
fn test_multiple_events(cx: &mut TestAppContext) {
    let events = Rc::new(RefCell::new(Vec::new()));

    let entity = cx.new({
        let events = events.clone();
        |cx| {
            cx.subscribe(&cx.entity(), move |_, _, event, _| {
                events.borrow_mut().push(event.clone());
            }).detach();
            MyEntity::new()
        }
    });

    // Perform actions that emit events
    entity.update(cx, |e, cx| {
        e.do_action_1(cx);
        e.do_action_2(cx);
    });

    // Verify events
    assert_eq!(events.borrow().len(), 2);
}
```
