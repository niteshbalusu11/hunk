---
title: Use h_flex() and v_flex() for Layouts
impact: MEDIUM
tags: flexbox, layout, h_flex, v_flex
---

## Flexbox Layout

GPUI uses Flexbox for layout with Tailwind-like method chaining.

### Convenience Methods

```rust
// Horizontal stack (flex + row + items_center)
.h_flex()

// Vertical stack (flex + col)
.v_flex()

// Global functions
h_flex()  // Creates div().h_flex()
v_flex()  // Creates div().v_flex()
```

### Basic Flexbox

```rust
div()
    .flex()              // Enable flex
    .flex_row()          // Horizontal (default)
    .flex_col()          // Vertical
    .flex_wrap()         // Allow wrapping

    // Cross-axis alignment
    .items_start()       // Align to start
    .items_center()      // Center
    .items_end()         // Align to end
    .items_stretch()     // Stretch to fill

    // Main-axis alignment
    .justify_start()
    .justify_center()
    .justify_end()
    .justify_between()   // Space between items
    .justify_around()    // Space around items

    // Gap
    .gap(px(8.))
    .gap_x(px(8.))       // Horizontal only
    .gap_y(px(8.))       // Vertical only
    .gap_2()             // 8px shorthand
```

### Flex Item Sizing

```rust
.flex_1()            // Grow + shrink, ignore initial size
.flex_auto()         // Grow + shrink, respect initial size
.flex_initial()      // Shrink only
.flex_none()         // Fixed size
.flex_grow()         // Allow grow
.flex_shrink()       // Allow shrink
```

### Common Layout Patterns

**Header with title and actions:**
```rust
h_flex()
    .justify_between()
    .items_center()
    .child(title)
    .child(
        h_flex()
            .gap_2()
            .child(edit_button)
            .child(close_button)
    )
```

**Scrollable content with fixed header/footer:**
```rust
v_flex()
    .size_full()
    .child(header)  // Fixed
    .child(
        div()
            .flex_1()
            .overflow_y_scroll()
            .child(content)
    )
    .child(footer)  // Fixed
```

**Centered content:**
```rust
div()
    .flex()
    .items_center()
    .justify_center()
    .size_full()
    .child(centered_content)
```
