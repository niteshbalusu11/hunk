---
title: Use Elevation System for Layered Surfaces
impact: MEDIUM
tags: elevation, shadow, layers
---

## Elevation System

Use elevation for consistent visual hierarchy across surfaces.

### Elevation Levels

```rust
pub enum ElevationIndex {
    Background,       // Below main surface
    Surface,          // Main panels
    EditorSurface,    // Editor area
    ElevatedSurface,  // Floating elements
    ModalSurface,     // Dialogs, modals
}
```

### Using Elevation

```rust
// Convenience methods
div().elevation_1(cx)  // Surface level
div().elevation_2(cx)  // Elevated surface
div().elevation_3(cx)  // Modal surface
```

Each elevation includes:
- Background color
- Border
- Box shadow

### Manual Shadow

```rust
.shadow(vec![
    BoxShadow {
        color: hsla(0., 0., 0., 0.12),
        offset: point(px(0.), px(2.)),
        blur_radius: px(3.),
        spread_radius: px(0.),
    }
])

// Shortcuts
.shadow_sm()
.shadow_md()
.shadow_lg()
```

### Practical Examples

**Card Component:**
```rust
div()
    .v_flex()
    .gap_y(px(12.))
    .p(px(16.))
    .elevation_2(cx)
    .rounded_lg()
    .child(title)
    .child(content)
```

**Modal Dialog:**
```rust
div()
    .v_flex()
    .w(px(400.))
    .max_h(px(600.))
    .p(px(24.))
    .elevation_3(cx)
    .rounded_xl()
    .child(header)
    .child(body)
    .child(footer)
```

**Dropdown Menu:**
```rust
div()
    .absolute()
    .elevation_2(cx)
    .rounded_md()
    .py_1()
    .children(menu_items)
```
