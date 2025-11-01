/// Core trait that any bounding box must implement to use XY-Cut++
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
}
