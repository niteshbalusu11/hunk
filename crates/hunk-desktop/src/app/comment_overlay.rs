pub(crate) fn review_comment_overlay_top_px(
    row_top_px: usize,
    scroll_top_px: usize,
    viewport_height_px: usize,
    row_height_px: usize,
) -> f32 {
    let desired_top = row_top_px
        .saturating_sub(scroll_top_px)
        .saturating_add(row_height_px) as f32;
    let max_top = viewport_height_px.saturating_sub(176) as f32;
    desired_top.clamp(8.0, max_top.max(8.0))
}
