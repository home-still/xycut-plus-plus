use crate::traits::BoundingBox;

/// Count how many elements the given element overlaps with
pub fn count_overlap<T: BoundingBox>(element: &T, all_elements: &[T]) -> usize {
    let (x1, y1, x2, y2) = element.bounds();

    all_elements
        .iter()
        .filter(|other| {
            // Don't count self
            if other.id() == element.id() {
                return false;
            }

            // Check if bounding boxes overlap in both X and Y
            let (ox1, oy1, ox2, oy2) = other.bounds();
            (x1 < ox2 && x2 > ox1) && (y1 < oy2 && y2 > oy1)
        })
        .count()
}

/// Calculate median width of elements
pub fn compute_median_width<T: BoundingBox>(elements: &[T]) -> f32 {
    if elements.is_empty() {
        return 0.0;
    }

    let mut widths: Vec<f32> = elements
        .iter()
        .map(|e| {
            let (x1, _, x2, _) = e.bounds();
            x2 - x1
        })
        .collect();

    widths.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let len = widths.len();
    if len % 2 == 1 {
        widths[len / 2]
    } else {
        (widths[len / 2 - 1] + widths[len / 2]) / 2.0
    }
}
