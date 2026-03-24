# Lessons Learned

- Focus restoration bug: Drawer, modal, or panel close paths can restore focus too early while the UI is still unmounting, which leaves the workspace without a stable focused target and causes keyboard shortcuts to stop working until the user clicks again.
  Fix: Capture the pre-open focus target and restore focus with a deferred action after the overlay or panel has fully closed.
- Context-sensitive focus bug: Temporary surfaces such as terminals or popovers can steal focus from an editor or workspace root and fail to return it to the correct place on close.
  Fix: Record where focus came from, such as an editor vs. a workspace container, and route every close path back to that same target.
