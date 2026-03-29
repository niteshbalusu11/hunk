mod data {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub(super) enum DiffSegmentQuality {
        #[default]
        Plain,
        SyntaxOnly,
        Detailed,
    }
}

#[path = "../src/app/diff_segment_prefetch.rs"]
mod diff_segment_prefetch;

use data::DiffSegmentQuality;
use diff_segment_prefetch::{first_paint_prefetch_window, prioritized_prefetch_row_indices};

#[test]
fn first_paint_window_biases_toward_rows_below_the_visible_top() {
    assert_eq!(first_paint_prefetch_window(200, 40, 32), (32, 64));
    assert_eq!(first_paint_prefetch_window(200, 3, 32), (0, 32));
}

#[test]
fn first_paint_window_backfills_near_the_end_of_the_diff() {
    assert_eq!(first_paint_prefetch_window(40, 38, 20), (20, 40));
}

#[test]
fn first_paint_quality_caps_detailed_files_at_syntax_only() {
    assert_eq!(
        diff_segment_prefetch::first_paint_segment_quality(DiffSegmentQuality::Detailed),
        DiffSegmentQuality::SyntaxOnly
    );
    assert_eq!(
        diff_segment_prefetch::first_paint_segment_quality(DiffSegmentQuality::SyntaxOnly),
        DiffSegmentQuality::SyntaxOnly
    );
}

#[test]
fn prioritized_prefetch_rows_start_at_anchor_then_expand_outward() {
    assert_eq!(
        prioritized_prefetch_row_indices(10, 16, 12),
        vec![12, 13, 11, 14, 10, 15]
    );
}
