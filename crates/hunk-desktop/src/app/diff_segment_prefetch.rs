use super::data::DiffSegmentQuality;

pub(crate) fn first_paint_prefetch_window(
    total_rows: usize,
    anchor_row: usize,
    window_rows: usize,
) -> (usize, usize) {
    if total_rows == 0 || window_rows == 0 {
        return (0, 0);
    }

    let clamped_anchor = anchor_row.min(total_rows.saturating_sub(1));
    let before_rows = window_rows / 4;
    let desired_start = clamped_anchor.saturating_sub(before_rows);
    let desired_end = desired_start.saturating_add(window_rows).min(total_rows);
    let filled_start = desired_end.saturating_sub(window_rows);

    (filled_start, desired_end)
}

pub(crate) fn first_paint_segment_quality(base_quality: DiffSegmentQuality) -> DiffSegmentQuality {
    match base_quality {
        DiffSegmentQuality::Plain => DiffSegmentQuality::Plain,
        DiffSegmentQuality::SyntaxOnly | DiffSegmentQuality::Detailed => {
            DiffSegmentQuality::SyntaxOnly
        }
    }
}

pub(crate) fn prioritized_prefetch_row_indices(
    start: usize,
    end: usize,
    anchor_row: usize,
) -> Vec<usize> {
    if start >= end {
        return Vec::new();
    }

    let anchor = anchor_row.clamp(start, end.saturating_sub(1));
    let mut rows = Vec::with_capacity(end.saturating_sub(start));
    rows.push(anchor);

    let mut step = 1usize;
    while rows.len() < end.saturating_sub(start) {
        let mut inserted = false;

        if let Some(right) = anchor.checked_add(step)
            && right < end
        {
            rows.push(right);
            inserted = true;
        }

        if let Some(left) = anchor.checked_sub(step)
            && left >= start
        {
            rows.push(left);
            inserted = true;
        }

        if !inserted {
            break;
        }

        step = step.saturating_add(1);
    }

    rows
}
