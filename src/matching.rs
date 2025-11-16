use crate::traits::BoundingBox;
use crate::utils::{compute_median_width, count_overlap, distance_to_nearest_text};

/// Isolation threshold in pixels for Equation 3.
///
/// Paper states φtext(Bi) = ∞ indicates "not adjacent to any text box"
/// but doesn't specify exact distance. 50px chosen empirically as reasonable
/// threshold for "non-adjacent" in typical document layouts.
///
/// Paper reference: Section 3.1, Equation 3
const ISOLATION_THRESHOLD_PX: f32 = 50.0;

/// Result of pre-mask processing
#[derive(Debug)]
pub struct MaskPartition<T: BoundingBox> {
    pub masked_elements: Vec<T>,
    pub regular_elements: Vec<T>,
}

/// Partition elements into masked titles, figures, tables and regular text
/// This is Step 1 of XY-Cut++: Pre-mask processing
// TODO: Add page_width parameter to function signature
pub fn partition_by_mask<T: BoundingBox>(
    elements: &[T],
    page_width: f32,
    page_height: f32,
) -> MaskPartition<T> {
    let mut masked_elements = Vec::new();
    let mut regular_elements = Vec::new();

    let median_width = compute_median_width(elements);
    let threshold = 1.3 * median_width;

    // Equation 3 - geometric pre-segmentation
    // Calculate page center
    let page_center_x = page_width / 2.0;
    let page_center_y = page_height / 2.0;

    // Calculate page diagonal for normalization
    let page_diagonal = (page_width * page_width + page_height * page_height).sqrt();

    for element in elements {
        // Also mask wide-spanning elements (>70% page width)
        // This helps column detection by removing elements that span both columns
        // Calculate element width from bounds and compare to page_width * 0.7

        let (x1, _, x2, _) = element.bounds();
        let width = x2 - x1;
        let overlap_count = count_overlap(element, elements);
        let is_cross_layout = width > threshold && overlap_count >= 2;

        // Equation 3 - check if element is central and isolated
        // (only for visual elements)
        let (cx, cy) = element.center();

        let distance_to_center =
            ((cx - page_center_x).powi(2) + (cy - page_center_y).powi(2)).sqrt();

        // Normalize by page diagonal
        let normalized_distance = distance_to_center / page_diagonal;

        // Check centrality (within 20% of page dimension)
        let is_central = normalized_distance <= 0.2;

        // Check isolation (no adjacent text within 50px)
        let dist_to_text = distance_to_nearest_text(element, elements);
        let is_isolated = dist_to_text > ISOLATION_THRESHOLD_PX;

        // Apply Equation 3 - mask if central AND isolated AND visual element
        let is_geometric_mask = is_central && is_isolated && element.should_mask();

        if element.should_mask() || is_cross_layout || is_geometric_mask {
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
