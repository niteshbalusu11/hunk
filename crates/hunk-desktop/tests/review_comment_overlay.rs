#[allow(dead_code)]
#[path = "../src/app/comment_overlay.rs"]
mod comment_overlay;

#[test]
fn review_comment_overlay_top_clamps_into_viewport() {
    let top_near_start = comment_overlay::review_comment_overlay_top_px(0, 0, 320, 26);
    assert_eq!(top_near_start, 26.0);

    let top_when_row_is_above_viewport =
        comment_overlay::review_comment_overlay_top_px(120, 240, 320, 26);
    assert_eq!(top_when_row_is_above_viewport, 26.0);

    let top_near_bottom = comment_overlay::review_comment_overlay_top_px(540, 320, 320, 26);
    assert_eq!(top_near_bottom, 144.0);
}
