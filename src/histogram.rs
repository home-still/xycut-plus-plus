use crate::traits::BoundingBox;

/// Build a horizontal projection histogram to find row gaps
/// Returns a histogram where bin counts how many elements overlap that y-coordinate
pub fn build_horizontal_histogram<T: BoundingBox>(
    elements: &[T],
    y_min: f32,
    y_max: f32,
    resolution: usize,
) -> Vec<usize> {
    let mut histogram = vec![0; resolution];
    let bin_height = (y_max - y_min) / resolution as f32;

    for element in elements {
        let (_, y1, _, y2) = element.bounds();
        let start_bin = ((y1 - y_min) / bin_height).floor().max(0.0) as usize;
        let end_bin = ((y2 - y_min) / bin_height).ceil().min(resolution as f32) as usize;

        for bin in start_bin..end_bin.min(resolution) {
            if bin < histogram.len() {
                histogram[bin] += 1;
            }
        }
    }

    histogram
}

/// Build a vertical projection histogram to find column gaps
/// Returns a histogram where each bin counts how many elements overlap that x-coordinate
pub fn build_vertical_histogram<T: BoundingBox>(
    elements: &[T],
    x_min: f32,
    x_max: f32,
    resolution: usize,
) -> Vec<usize> {
    let mut histogram = vec![0; resolution];
    let bin_width = (x_max - x_min) / resolution as f32;

    for element in elements {
        let (x1, _, x2, _) = element.bounds();
        let start_bin = ((x1 - x_min) / bin_width).floor().max(0.0) as usize;
        let end_bin = ((x2 - x_min) / bin_width).ceil().min(resolution as f32) as usize;

        // TODO: Add bounds checking to prevent panic
        // Change to: if bin < histogram.len() { histogram[bin] += 1; }

        // TEMPORARY: Unsafe array access
        for bin in start_bin..end_bin.min(resolution) {
            if bin < histogram.len() {
                histogram[bin] += 1;
            }
        }
    }

    histogram
}

/// Find the largest gap in a histogram (consecutive bins with 0 count)
/// Returns the center position of the largest gap, or None if no gap found
pub fn find_largest_gap(histogram: &[usize], min_gap_size: usize) -> Option<usize> {
    let mut max_gap_size = 0;
    let mut max_gap_center = None;
    let mut current_gap_size = 0;
    let mut current_gap_start = None;

    for (i, &count) in histogram.iter().enumerate() {
        if count == 0 {
            // In a gap
            if current_gap_start.is_none() {
                current_gap_start = Some(i);
            }

            current_gap_size += 1;
        } else {
            // End of gap
            if current_gap_size >= min_gap_size && current_gap_size > max_gap_size {
                max_gap_size = current_gap_size;
                if let Some(start) = current_gap_start {
                    max_gap_center = Some(start + current_gap_size / 2);
                }
                current_gap_size = 0;
                current_gap_start = None
            }
        }
    }

    // Check the last gap
    if current_gap_size >= min_gap_size && current_gap_size > max_gap_size {
        if let Some(start) = current_gap_start {
            max_gap_center = Some(start + current_gap_size / 2);
        }
    }

    max_gap_center
}
