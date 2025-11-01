use core::f32;

use crate::histogram::{build_horizontal_histogram, build_vertical_histogram, find_largest_gap};
use crate::matching::partition_by_mask;
use crate::traits::BoundingBox;

/// Configuration for XY-Cut algorithm
#[derive(Debug, Clone)]
pub struct XYCutConfig {
    /// Minimum gap size (in pixels) to consider for cutting
    pub min_cut_threshold: f32,

    /// Resolution for projection histogram (bin per 100 pixels)
    pub histogram_resolution_scale: f32,

    /// Tolerance for considering elements in the same row (pixels)
    pub same_row_tolerance: f32,
}

impl Default for XYCutConfig {
    fn default() -> Self {
        Self {
            min_cut_threshold: 15.0,
            histogram_resolution_scale: 0.5, // 1 bin per 2 pixels
            same_row_tolerance: 10.0,
        }
    }
}

pub struct XYCut {
    config: XYCutConfig,
}

impl XYCut {
    pub fn new(config: XYCutConfig) -> Self {
        Self { config }
    }

    /// Main entry point: compute reading order for elements
    pub fn compute_order<T: BoundingBox>(
        &self,
        elements: &[T],
        x_min: f32,
        y_min: f32,
        x_max: f32,
        y_max: f32,
    ) -> Vec<usize> {
        let page_width = x_max - x_min;
        let partition = partition_by_mask(elements, page_width);
        let regular_order =
            self.recursive_cut(&partition.regular_elements, x_min, y_min, x_max, y_max);

        self.merged_masked_elements(
            &partition.regular_elements,
            &regular_order,
            &partition.masked_elements,
        )
    }

    fn recursive_cut<T: BoundingBox>(
        &self,
        elements: &[T],
        x_min: f32,
        y_min: f32,
        x_max: f32,
        y_max: f32,
    ) -> Vec<usize> {
        if elements.is_empty() {
            return Vec::new();
        }
        if elements.len() == 1 {
            return vec![elements[0].id()];
        }

        // For large groups, try vertical cut first to detect columns
        // Academic papers often have 2 columns, and the column gap is significant
        let try_vertical_first = elements.len() > 10;

        if try_vertical_first {
            // Try vertical cut first for multi-column layouts
            if let Some(x_cut) = self.find_vertical_cut(elements, x_min, x_max) {
                eprintln!(
                    "  [XYCut] Vertical cut at x={:.0}, splitting {} elements (multi-column)",
                    x_cut,
                    elements.len()
                );
                let (left, right) = self.split_vertical(elements, x_cut);
                eprintln!(
                    "    → Left: {} elements, Right: {} elements",
                    left.len(),
                    right.len()
                );
                let mut result = Vec::new();
                result.extend(self.recursive_cut(&left, x_min, y_min, x_cut, y_max));
                result.extend(self.recursive_cut(&right, x_cut, y_min, x_max, y_max));
                return result;
            }
        }

        // Try horizontal cut first (top-to-bottom reading)
        if let Some(y_cut) = self.find_horizontal_cut(elements, y_min, y_max) {
            eprintln!(
                "  [XYCut] Horizontal cut at y={:.0}, splitting {} elements",
                y_cut,
                elements.len()
            );
            let (top, bottom) = self.split_horizontal(elements, y_cut);
            eprintln!(
                "    → Top: {} elements, Bottom: {} elements",
                top.len(),
                bottom.len()
            );
            let mut result = Vec::new();
            result.extend(self.recursive_cut(&top, x_min, y_min, x_max, y_cut));
            result.extend(self.recursive_cut(&bottom, x_min, y_cut, x_max, y_max));
            return result;
        }

        // Try vertical cut (left-to-right for multi-column)
        if let Some(x_cut) = self.find_vertical_cut(elements, x_min, x_max) {
            eprintln!(
                "  [XYCut] Vertical cut at x={:.0}, splitting {} elements",
                x_cut,
                elements.len()
            );
            let (left, right) = self.split_vertical(elements, x_cut);
            eprintln!(
                "    → Left: {} elements, Right: {} elements",
                left.len(),
                right.len()
            );
            let mut result = Vec::new();
            result.extend(self.recursive_cut(&left, x_min, y_min, x_cut, y_max));
            result.extend(self.recursive_cut(&right, x_cut, y_min, x_max, y_max));
            return result;
        }

        // No valid cuts found - sort by position
        eprintln!(
            "  [XYCut] No cuts found, sorting {} elements by position",
            elements.len()
        );
        self.sort_by_position(elements)
    }

    /// Find horizontal cut position using projection histogram
    /// Returns y-coordinate where to split, or None if no good cut found
    fn find_horizontal_cut<T: BoundingBox>(
        &self,
        elements: &[T],
        y_min: f32,
        y_max: f32,
    ) -> Option<f32> {
        let resolution = ((y_max - y_min) * self.config.histogram_resolution_scale) as usize;
        let histogram = build_horizontal_histogram(elements, y_min, y_max, resolution);

        let min_gap_bins =
            (self.config.min_cut_threshold * self.config.histogram_resolution_scale) as usize;

        let bin_index = find_largest_gap(&histogram, min_gap_bins);

        if let Some(bin_index) = bin_index {
            let y_coord = y_min + (bin_index as f32 / resolution as f32) * (y_max - y_min);
            return Some(y_coord);
        }

        None
    }

    /// Find vertical cut position using projection histogram
    /// Returns x-coordinate where to split, or None if no good cut found
    fn find_vertical_cut<T: BoundingBox>(
        &self,
        elements: &[T],
        x_min: f32,
        x_max: f32,
    ) -> Option<f32> {
        let resolution = ((x_max - x_min) * self.config.histogram_resolution_scale) as usize;
        let histogram = build_vertical_histogram(elements, x_min, x_max, resolution);

        let min_gap_bins =
            (self.config.min_cut_threshold * self.config.histogram_resolution_scale) as usize;

        // Debug: show histogram for large element counts
        if elements.len() > 15 {
            eprintln!(
                "    [Histogram] Vertical: {} bins, min_gap={}, x_range={:.0}-{:.0}",
                resolution, min_gap_bins, x_min, x_max
            );
        }

        let bin_index = find_largest_gap(&histogram, min_gap_bins);
        if let Some(bin_index) = bin_index {
            let x_coord = x_min + (bin_index as f32 / resolution as f32) * (x_max - x_min);
            if elements.len() > 15 {
                eprintln!(
                    "    [Histogram] Found gap at bin {}, x={:.0}",
                    bin_index, x_coord
                );
            }
            return Some(x_coord);
        }

        None
    }

    /// Split elements into top and bottom groups based on y-coordinate cut
    fn split_horizontal<T: BoundingBox>(&self, elements: &[T], y_cut: f32) -> (Vec<T>, Vec<T>) {
        let mut top: Vec<T> = Vec::new();
        let mut bottom: Vec<T> = Vec::new();

        for element in elements.iter() {
            if element.center().1 < y_cut {
                top.push(element.clone());
            } else {
                bottom.push(element.clone())
            }
        }

        (top, bottom)
    }

    /// Split elements into left and right groups based on x-coordinate cut
    fn split_vertical<T: BoundingBox>(&self, elements: &[T], x_cut: f32) -> (Vec<T>, Vec<T>) {
        let mut left: Vec<T> = Vec::new();
        let mut right: Vec<T> = Vec::new();

        for element in elements.iter() {
            if element.center().0 < x_cut {
                left.push(element.clone());
            } else {
                right.push(element.clone());
            }
        }

        (left, right)
    }

    /// Fallback sorting when no valid cuts found
    /// Sort by y-position first (top to bottom), then x-position (left to right)
    fn sort_by_position<T: BoundingBox>(&self, elements: &[T]) -> Vec<usize> {
        let mut indexed: Vec<(usize, T)> = elements
            .iter()
            .enumerate()
            .map(|(i, bbox)| (i, bbox.clone()))
            .collect();

        indexed.sort_by(|a, b| {
            let y_diff = (a.1.center().1 - b.1.center().1).abs();
            if y_diff < self.config.same_row_tolerance {
                // Same row - sort by x
                a.1.center().0.partial_cmp(&b.1.center().0).unwrap()
            } else {
                // Different rows - sort by y
                a.1.center().1.partial_cmp(&b.1.center().1).unwrap()
            }
        });

        indexed.iter().map(|(_, bbox)| bbox.id()).collect()
    }

    fn compute_page_width<T: BoundingBox>(&self, elements: &[T]) -> f32 {
        if elements.is_empty() {
            return 0.0;
        }
        let x_min = elements
            .iter()
            .map(|e| e.bounds().0)
            .fold(f32::INFINITY, f32::min);
        let x_max = elements
            .iter()
            .map(|e| e.bounds().2)
            .fold(f32::NEG_INFINITY, f32::max);

        x_max - x_min
    }

    fn merged_masked_elements<T: BoundingBox>(
        &self,
        regular_elements: &[T],
        regular_order: &[usize],
        masked_elements: &[T],
    ) -> Vec<usize> {
        // Start with regular order as base
        let mut result: Vec<usize> = regular_order.to_vec();

        // This ensures top elements (like page titles) are inserted first
        // Sort by y first (top-to-bottom), then x (left-to-right) for same row
        let mut sort_masked: Vec<T> = masked_elements.to_vec();
        sort_masked.sort_by(|a, b| {
            let y_diff = (a.center().1 - b.center().1).abs();
            if y_diff < self.config.same_row_tolerance {
                // Elements are in same row, sort by x (left-to-right)
                a.center().0.partial_cmp(&b.center().0).unwrap()
            } else {
                // Different rows, sort by y (top-to-bottom)
                a.center().1.partial_cmp(&b.center().1).unwrap()
            }
        });

        // For each masked element, find where to insert it
        for masked in &sort_masked {
            // Find best insertion position by checking IoU with regular elements
            let mut best_iou = 0.0;
            let mut best_position = 0;

            for (i, regular_id) in regular_order.iter().enumerate() {
                // Find the regular element by id
                if let Some(regular) = regular_elements.iter().find(|e| e.id() == *regular_id) {
                    let iou = masked.iou(regular);
                    if iou > best_iou {
                        best_iou = iou;
                        best_position = i;
                    }
                }
            }

            // Insert masked element at best position
            // If IoU > 0, insert after the matched element
            // Otherwise, insert based on y-position
            if best_iou > 0.0 {
                result.insert(best_position + 1, masked.id());
            } else {
                // No overlap - find insertion by y-position AND x-position (column-aware)
                let (masked_x, masked_y) = masked.center();

                // Get masked element bounds and width
                let masked_bounds = masked.bounds();
                let masked_width = masked_bounds.2 - masked_bounds.0;

                // Check if this is a wide spanning element (>60% page width)
                let page_width = self.compute_page_width(regular_elements);
                let is_spanning = masked_width > (page_width * 0.6);

                let mut insert_pos = result.len();

                // Iterate over result instead of regular_order
                // This is important because result changes as we insert elements
                for (i, regular_id) in result.iter().enumerate() {
                    if let Some(regular) = regular_elements.iter().find(|e| e.id() == *regular_id) {
                        let (regular_x, regular_y) = regular.center();

                        // If is_spanning, only check y-position
                        if is_spanning {
                            if regular_y > masked_y {
                                insert_pos = i;
                                break;
                            }
                        } else {
                            // Use LEFT EDGE distance instead of center distance
                            // This fixes column detection for boxes with different widths
                            let masked_left = masked.bounds().0;
                            let regular_left = regular.bounds().0;
                            let same_column = (regular_left - masked_left).abs() < 100.0;

                            // Replace above with column-aware check
                            if same_column && regular_y > masked_y {
                                insert_pos = i;
                                break;
                            }
                        }
                    }
                }
                result.insert(insert_pos, masked.id());
            }
        }
        result
    }
}
