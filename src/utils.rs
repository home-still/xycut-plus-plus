use crate::traits::{BoundingBox, SemanticLabel};
use core::f32;

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

/// Optimized distance calculation with early termination (Algorithm 1)
/// Returns early if partial distance exceeds current_best
pub fn compute_distance_with_early_exit<T: BoundingBox>(
    masked: &T,
    regular: &T,
    current_best: f32,
) -> f32 {
    let (mx1, my1, mx2, my2) = masked.bounds();
    let (rx1, ry1, rx2, ry2) = regular.bounds();

    // Derive cross-layout behavior from semantic label
    let is_cross_layout = matches!(masked.semantic_label(), SemanticLabel::CrossLayout);

    // Calculate dimensions abd base weights
    let mw = mx2 - mx1;
    let mh = my2 - my1;
    let max_dim = mw.max(mh);

    // Detect element orientation from aspect ratio
    let is_horizontal = mw > mh;

    let base_w1 = max_dim * max_dim;
    let base_w2 = max_dim;
    let base_w3 = 1.0;
    let base_w4 = 1.0 / max_dim;

    // Paper reference: Section 3.2, page 5, Table 2
    // Weights determined from grid search on 2.8k documents
    let label = masked.semantic_label();
    let (mult_w1, mult_w2, mult_w3, mult_w4) = match label {
        // Lcross-layout: [1, 1, 0.1, 1]
        SemanticLabel::CrossLayout => (1.0, 1.0, 0.1, 1.0),

        // Ltitle: Check ACTUAL orientation (not semantic label name)
        // Paper uses intersection: Ltitle ∩ Ohoriz and Ltitle ∩ Overt
        SemanticLabel::HorizontalTitle | SemanticLabel::VerticalTitle => {
            if is_horizontal {
                // Ltitle ∩ Ohoriz: [1, 0.1, 0.1, 1]
                (1.0, 0.1, 0.1, 1.0)
            } else {
                // Ltitle ∩ Overt: [0.2, 0.1, 1, 1]
                (0.2, 0.1, 1.0, 1.0)
            }
        }

        // Lotherwise: [1, 1, 1, 0.1]
        // Applies to Vision, Regular, and all other cases
        _ => (1.0, 1.0, 1.0, 0.1),
    };

    // Apply semantic multipliers to base weights
    let w1 = base_w1 * mult_w1;
    let w2 = base_w2 * mult_w2;
    let w3 = base_w3 * mult_w3;
    let w4 = base_w4 * mult_w4;

    // Component-by-component calculation with early exist
    let mut distance = 0.0;

    // Component 1 (ϕ1): Intersection constraint
    let boxes_overlap = (mx1 < rx2 && mx2 > rx1) && (my1 < ry2 && my2 > ry1);
    let phi1 = if boxes_overlap { 0.0 } else { 100.0 };
    distance += w1 * phi1;
    if distance > current_best {
        return distance;
    }

    // Component 2 (ϕ2): Boundary proximity
    let dx = if mx2 < rx1 {
        rx1 - mx2 // Masked is to the left
    } else if mx1 > rx2 {
        mx1 - rx2 // Masked is to the right
    } else {
        0.0 // Boxes overlap horizontally
    };

    let dy = if my2 < ry1 {
        ry1 - my2 // Masked is above
    } else if my1 > ry2 {
        my1 - ry2 // Masked is below
    } else {
        0.0 // Boxes overlap vertically
    };

    let phi2 = if is_cross_layout {
        dx + dy // Diagonal distance for cross-layout
    } else {
        dx.min(dy) // Axis-aligned distance for single-column
    };
    distance += w2 * phi2;
    if distance > current_best {
        return distance;
    }

    // Component 3 (ϕ3): Vertical continuity
    let phi3 = if is_cross_layout {
        // Cross-layout: Prefer elements above current position
        if my1 > ry2 {
            my1 - ry2 // Masked is below regular - penalize
        } else {
            -my2 // Masked is above or overlaps - prefer higher position
        }
    } else {
        // Single column: Prefer elements below (reading flow)
        if ry1 >= my2 {
            ry1 - my1 // Regular below - baseline alignment (top-to-top)
        } else {
            (my2 - ry1) * 10.0 // Regular above - scaled penalty
        }
    };

    distance += w3 * phi3;
    if distance > current_best {
        return distance;
    }

    // Component 4 (ϕ4): Horizontal ordering
    let phi4 = rx1;
    distance + w4 * phi4
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

    widths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = widths.len();
    if len % 2 == 1 {
        widths[len / 2]
    } else {
        (widths[len / 2 - 1] + widths[len / 2]) / 2.0
    }
}

pub fn distance_to_nearest_text<T: BoundingBox>(element: &T, all_elements: &[T]) -> f32 {
    let mut min_distance = f32::INFINITY;
    // i love my dad
    let (mx1, my1, mx2, my2) = element.bounds();

    for other in all_elements {
        // Skip if same element
        if element.id() == other.id() {
            continue;
        }

        // Skip if not a text element
        if other.should_mask() {
            continue;
        }

        // Get other element's bounds
        let (tx1, ty1, tx2, ty2) = other.bounds();

        // Calculate horizontal distance (dx)
        // Component 2 (ϕ2): Boundary proximity
        let dx = if mx2 < tx1 {
            tx1 - mx2 // Masked is to the left
        } else if mx1 > tx2 {
            mx1 - tx2 // Masked is to the right
        } else {
            0.0 // Boxes overlap horizontally
        };

        let dy = if my2 < ty1 {
            ty1 - my2 // Masked is above
        } else if my1 > ty2 {
            my1 - ty2 // Masked is below
        } else {
            0.0 // Boxes overlap vertically
        };

        let euclidean_distance = (dx.powf(2.0) + dy.powf(2.0)).sqrt();

        if euclidean_distance < min_distance {
            min_distance = euclidean_distance
        }
    }

    min_distance
}
