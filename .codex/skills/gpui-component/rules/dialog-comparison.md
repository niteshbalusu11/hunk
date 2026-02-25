---
title: Dialog vs Popover vs PopupMenu vs Sheet
impact: HIGH
tags: dialog, popover, popup, sheet
---

## Popup Component Comparison

Choose the right overlay component for your use case.

### Visual Comparison

```
Dialog                    Popover                   PopupMenu
┌─────────────┐          ┌─────────────┐           ┌─────────────┐
│  ┌───────┐  │          │   Trigger   │           │   Trigger   │
│  │ Title │  │          │      ↓      │           │      ↓      │
│  ├───────┤  │          │ ┌─────────┐ │           │ ┌─────────┐ │
│  │ Body  │  │          │ │ Content │ │           │ │ Item 1  │ │
│  ├───────┤  │          │ │  (any)  │ │           │ ├─────────┤ │
│  │Footer │  │          │ └─────────┘ │           │ │ Item 2  │ │
│  └───────┘  │          └─────────────┘           │ │ Submenu→│ │
└─────────────┘                                    └─────────────┘
Centered + Overlay       Anchored to trigger       Menu items

Sheet
┌───────────────────────────────────────┬─────────┐
│                                       │ ┌─────┐ │
│          Main Content                 │ │Sheet│ │
│                                       │ │     │ │
└───────────────────────────────────────┴─────────┘
Side panel
```

### Comparison Table

| Feature | Dialog | Popover | PopupMenu | Sheet |
|---------|--------|---------|-----------|-------|
| **Position** | Centered | Anchored | Anchored | Side |
| **Overlay** | Yes (optional) | No | No | Yes (optional) |
| **State** | Root managed | keyed_state | Entity | Root managed |
| **Stacking** | Multiple | Single | Submenus | Single |
| **Close** | ESC/Click/Button | ESC/Outside | ESC/Select | ESC/Overlay |
| **Focus** | Auto-trapped | Optional | Auto-trapped | Auto-trapped |
| **Animation** | Fade+Slide | None | None | Slide |
| **Keyboard** | Enter/ESC | ESC | ↑↓←→/Enter/ESC | ESC |

### When to Use

| Component | Use For | Don't Use For |
|-----------|---------|---------------|
| **Dialog** | Confirmations, forms, important info | Simple tips, menus |
| **Popover** | Tooltips, rich previews, small forms | Complex interactions |
| **PopupMenu** | Context menus, dropdowns, actions | Forms, complex content |
| **Sheet** | Detail panels, settings, side nav | Quick confirmations |

### API Comparison

```rust
// Dialog - via WindowExt
window.open_dialog(cx, |dialog, _, _| {
    dialog.title("Title").child("Content")
});

// Popover - inline declaration
Popover::new("popover-id")
    .trigger(Button::new("trigger").label("Click"))
    .content(|_, _, _| div().child("Content"))

// PopupMenu - Entity + ContextMenu
let menu = PopupMenu::build(window, cx, |menu, _, _| {
    menu.menu("Action", Box::new(MyAction))
});
ContextMenu::build(window, cx).menu(menu);

// Sheet - via WindowExt
window.open_sheet(cx, |sheet, _, _| {
    sheet.title("Panel").child("Content")
});
```

### Decision Tree

```
Need modal blocking?
├─ Yes → Need confirm/cancel?
│        ├─ Yes → Dialog.confirm()
│        └─ No → Is it side content?
│                ├─ Yes → Sheet
│                └─ No → Dialog
└─ No → Is it a menu/options?
        ├─ Yes → PopupMenu
        └─ No → Popover
```
