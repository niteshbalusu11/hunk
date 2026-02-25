---
title: GPUI Testing Fundamentals
impact: HIGH
tags: testing, test-context, gpui-test
---

## GPUI Test Framework

GPUI provides a deterministic test framework for reliable async testing.

### Basic Test Structure

```rust
#[gpui::test]
async fn test_example(cx: &mut TestAppContext) {
    // 1. Initialize
    init_test(cx, |_| {});

    // 2. Create entities
    let entity = cx.new(|cx| MyEntity::new(cx));

    // 3. Create window if needed
    let (view, mut cx) = cx.add_window_view(|window, cx| {
        MyView::new(window, cx)
    });

    // 4. Perform actions
    view.update(&mut cx, |view, window, cx| {
        view.do_something(window, cx);
    });

    // 5. Wait for tasks to complete
    cx.run_until_parked();

    // 6. Assert results
    assert_eq!(...);
}
```

### Test Macro Parameters

```rust
// Basic async test
#[gpui::test]
async fn test_basic(cx: &mut TestAppContext) { }

// Multiple test contexts (collaboration testing)
#[gpui::test]
async fn test_collab(cx_a: &mut TestAppContext, cx_b: &mut TestAppContext) { }

// Fixed random seed
#[gpui::test(seed = 42)]
async fn test_deterministic(cx: &mut TestAppContext) { }

// Multiple iterations with different seeds
#[gpui::test(iterations = 10)]
async fn test_random(cx: &mut TestAppContext) { }

// With random number generator
#[gpui::test]
async fn test_with_rng(cx: &mut TestAppContext, rng: StdRng) { }
```

### Context Types

| Context | Use Case |
|---------|----------|
| `TestAppContext` | Basic entity testing |
| `VisualTestContext` | Window and UI testing |
| `EditorTestContext` | Editor-specific testing |
| `EditorLspTestContext` | Editor + LSP testing |

### Creating Windows

```rust
// With view
let (view, cx) = cx.add_window_view(|window, cx| {
    MyView::new(window, cx)
});
// cx is now VisualTestContext

// Without view
let window = cx.add_window(|window, cx| {
    // setup
});
```
