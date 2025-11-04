use crate::traits::BoundingBox;
use crate::utils::{compute_median_width, count_overlap};

/// Result of pre-mask processing
#[derive(Debug)]
pub struct MaskPartition<T: BoundingBox> {
    pub masked_elements: Vec<T>,
    pub regular_elements: Vec<T>,
}

/// Partition elements into masked titles, figures, tables and regular text
/// This is Step 1 of XY-Cut++: Pre-mask processing
// TODO: Add page_width parameter to function signature
pub fn partition_by_mask<T: BoundingBox>(elements: &[T], page_width: f32) -> MaskPartition<T> {
    let mut masked_elements = Vec::new();
    let mut regular_elements = Vec::new();

    let median_width = compute_median_width(elements);
    let threshold = 1.3 * median_width;

    for element in elements {
        // Also mask wide-spanning elements (>70% page width)
        // This helps column detection by removing elements that span both columns
        // Calculate element width from bounds and compare to page_width * 0.7

        let (x1, _, x2, _) = element.bounds();
        let width = x2 - x1;
        let overlap_count = count_overlap(element, elements);
        let is_cross_layout = width > threshold && overlap_count >= 2;

        if element.should_mask() || is_cross_layout {
            masked_elements.push(element.clone());
        } else {
            regular_elements.push(element.clone());
        }
    }

    MaskPartition {
        masked_elements,
        regular_elements,
    }
}
