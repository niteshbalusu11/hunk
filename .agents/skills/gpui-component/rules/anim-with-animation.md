---
title: Use with_animation for Transitions
impact: MEDIUM
tags: animation, transition, effects
---

## Animation Pattern

Use `with_animation` for smooth transitions in gpui-component.

### Basic Animation

```rust
let animation = Animation::new(Duration::from_secs_f64(0.25))
    .with_easing(cubic_bezier(0.32, 0.72, 0., 1.));

div()
    .with_animation("fade-in", animation, move |this, delta| {
        // delta goes from 0.0 to 1.0
        this.opacity(delta)
    })
```

### Dialog Entry Animation

```rust
impl RenderOnce for Dialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let animation = Animation::new(Duration::from_secs_f64(0.25))
            .with_easing(cubic_bezier(0.32, 0.72, 0., 1.));

        let y = self.margin_top + (self.layer_ix as f32 * 16.0);

        div()
            // Slide down animation
            .with_animation("slide-down", animation.clone(), move |this, delta| {
                this.top(y * delta)
            })
            // Fade in animation
            .with_animation("fade-in", animation, move |this, delta| {
                this.opacity(delta)
            })
    }
}
```

### State-Triggered Animation

Use `use_keyed_state` to track previous state and trigger animations on change:

```rust
fn checkbox_check_icon(
    id: ElementId,
    checked: bool,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    // Persist previous state
    let toggle_state = window.use_keyed_state(id, cx, |_, _| checked);

    svg()
        .path(IconName::Check.path())
        .map(|this| {
            // Only animate when state changes
            if checked != *toggle_state.read(cx) {
                // Update state after animation delay
                cx.spawn({
                    let toggle_state = toggle_state.clone();
                    async move |cx| {
                        cx.background_executor()
                            .timer(Duration::from_secs_f64(0.25))
                            .await;
                        _ = toggle_state.update(cx, |this, _| *this = checked);
                    }
                }).detach();

                // Apply animation
                this.with_animation(
                    ElementId::NamedInteger("toggle".into(), checked as u64),
                    Animation::new(Duration::from_secs_f64(0.25)),
                    move |this, delta| {
                        this.opacity(if checked { delta } else { 1.0 - delta })
                    },
                )
            } else {
                this.into_any_element()
            }
        })
}
```

### Common Easing Functions

```rust
// Smooth ease out
cubic_bezier(0.32, 0.72, 0., 1.)

// Ease in-out
cubic_bezier(0.4, 0., 0.2, 1.)

// Bounce effect
cubic_bezier(0.68, -0.55, 0.265, 1.55)

// Linear (default)
Easing::linear()
```

### Animation Properties

```rust
Animation::new(duration)
    .with_easing(easing)      // Easing function
    .with_delay(delay)        // Start delay
    .repeat()                 // Loop forever
    .repeat_times(n)          // Repeat n times
```
