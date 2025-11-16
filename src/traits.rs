#[derive(Debug, Clone, Copy)]
pub enum SemanticLabel {
    CrossLayout,
    HorizontalTitle,
    VerticalTitle,
    Vision,
    Regular,
}

/// Core trait that any bounding box must implement to use XY-Cut++
///
/// # Paper Reference
/// Corresponds to Equation 6 atomic region representation:
/// ```text
/// Ri = ⟨x₁⁽ⁱ⁾, y₁⁽ⁱ⁾, x₂⁽ⁱ⁾, y₂⁽ⁱ⁾, Ci⟩, Ci ∈ Ctype
/// ```
/// where Ctype distinguishes Cross-layout (spanning multiple grid units)
/// and Single-layout (contained within one grid unit) components.
///
/// Paper reference: Section 3.1, Equation 6, page 4
pub trait BoundingBox: Clone {
    /// Returns unique identifier for this element
    fn id(&self) -> usize;

    /// Returns center point (x, y)
    fn center(&self) -> (f32, f32);

    /// Returns bounding box as (x1, y1, x2, y2)
    fn bounds(&self) -> (f32, f32, f32, f32);

    /// Calculate Intersection over Union with another box
    fn iou(&self, other: &Self) -> f32;

    /// Whether element should be masked (titles, figures, tables)
    fn should_mask(&self) -> bool;

    /// Returns the semantic label type for this element
    fn semantic_label(&self) -> SemanticLabel;
}
