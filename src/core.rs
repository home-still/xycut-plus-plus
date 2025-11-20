use core::f32;

use crate::histogram::{build_horizontal_histogram, build_vertical_histogram, find_largest_gap};
use crate::matching::partition_by_mask;
use crate::traits::{BoundingBox, SemanticLabel};
use crate::utils::compute_distance_with_early_exit;

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

pub struct XYCutPlusPlus {
    config: XYCutConfig,
}

impl XYCutPlusPlus {
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
        // Validate empty input
        if elements.is_empty() {
            return Vec::new();
        }

        let page_width = x_max - x_min;
        let page_height = y_max - y_min;

        // Validate page dimensions
        if !page_width.is_finite()
            || !page_height.is_finite()
            || page_width <= 0.0
            || page_height <= 0.0
        {
            eprintln!(
                "Warning: Invalid page dimensions ({}, {})",
                page_width, page_height
            );

            return Vec::new();
        }

        let partition = partition_by_mask(elements, page_width, page_height);
        let regular_order =
            self.recursive_cut(&partition.regular_elements, x_min, y_min, x_max, y_max);

        self.merged_masked_elements(
            &partition.regular_elements,
            &regular_order,
            &partition.masked_elements,
        )
    }

    // TODO: Add this function before recursive_cut
    /// Calculate density ratio τd (tau_d) from Equation 4-5
    /// τd = Σ(w_k^(Cc) / h_k^(Cc)) / Σ(w_k^(Cs) / h_k^(Cs))
    fn compute_density_ratio<T: BoundingBox>(elements: &[T]) -> f32 {
        let mut cross_layout_density = 0.0; // Cc - wide elements
        let mut single_layout_density = 0.0; // Cs - narrow elements

        for element in elements {
            let (x1, y1, x2, y2) = element.bounds();
            let width = x2 - x1;
            let height = y2 - y1;

            // Avoid division by zero
            if height == 0.0 {
                continue;
            }

            let aspect_ratio = width / height;

            // Use semantic label instead of width threshold
            match element.semantic_label() {
                SemanticLabel::CrossLayout => cross_layout_density += aspect_ratio,
                _ => single_layout_density += aspect_ratio,
            }
        }

        // Return the ratio τd = cross_layout_density / single_layout_density
        // Handle division by zero: if single_layout_density == 0.0, return 1.0
        if single_layout_density == 0.0 {
            return 1.0;
        }

        cross_layout_density / single_layout_density
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

        // Equation 4: Calculate density ration τd
        let tau_d = Self::compute_density_ratio(elements);

        // Equation 5: Use XY-Cut (vertical first) if τd > 0.9
        let try_vertical_first = tau_d > 0.9;

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
                a.1.center()
                    .0
                    .partial_cmp(&b.1.center().0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                // Different rows - sort by y
                a.1.center()
                    .1
                    .partial_cmp(&b.1.center().1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        indexed.iter().map(|(_, bbox)| bbox.id()).collect()
    }

    fn merged_masked_elements<T: BoundingBox>(
        &self,
        regular_elements: &[T],
        regular_order: &[usize],
        masked_elements: &[T],
    ) -> Vec<usize> {
        // Start with regular order as base
        let mut result: Vec<usize> = regular_order.to_vec();

        let mut priority_groups: Vec<Vec<T>> = vec![Vec::new(); 4];
        for element in masked_elements {
            let priority = Self::label_priority(element.semantic_label()) as usize;
            if priority < 4 {
                priority_groups[priority].push(element.clone());
            }
        }

        // Process each priority group in order (CrossLayout → Title → Vision → Regular)
        for mut group in priority_groups {
            // Within each priority group, sort by reading order (y, then x)
            group.sort_by(|a, b| {
                let y_diff = (a.center().1 - b.center().1).abs();
                if y_diff < self.config.same_row_tolerance {
                    a.center()
                        .0
                        .partial_cmp(&b.center().0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    a.center()
                        .1
                        .partial_cmp(&b.center().1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
            });

            // Process each element in this priority group
            for masked in &group {
                // Find the best insertion position using 4-component distance metric
                let mut best_distance = f32::INFINITY;
                let mut best_position: Option<usize> = None;

                // Get masked element's semantic priority for constraint checking
                let masked_priority = Self::label_priority(masked.semantic_label());

                // Search through result to handle growing array correctly
                for (idx, &elem_id) in result.iter().enumerate() {
                    // Find the element - could be regular OR previously inserted masked
                    let candidate = regular_elements
                        .iter()
                        .find(|e| e.id() == elem_id)
                        .cloned()
                        .or_else(|| {
                            // Also check masked elements from ALL groups
                            masked_elements.iter().find(|e| e.id() == elem_id).cloned()
                        });

                    if let Some(candidate) = candidate {
                        // Enforce L'o ⪰ l constraint (Equation 7)
                        let candidate_priority = Self::label_priority(candidate.semantic_label());
                        if candidate_priority < masked_priority {
                            continue;
                        }

                        // Use 4-component distance metric
                        let distance =
                            compute_distance_with_early_exit(masked, &candidate, best_distance);
                        if distance < best_distance {
                            best_distance = distance;
                            best_position = Some(idx);
                        }
                    }
                }

                if let Some(position) = best_position {
                    eprintln!(
                        "  [INSERT] Masked element {} ({:?}) -> position {} (before element {})",
                        masked.id(),
                        masked.semantic_label(),
                        position,
                        result[position]
                    );
                    result.insert(position, masked.id());
                } else {
                    // No valid match found - append to end as a fallback
                    eprintln!(
                        "⚠️  No valid insertion for element {} ({:?}), appending",
                        masked.id(),
                        masked.semantic_label()
                    );
                    result.push(masked.id());
                }
            }
        }
        result
    }

    /// Get priority value for semantic label (lower = higher priority)
    fn label_priority(label: SemanticLabel) -> u8 {
        match label {
            SemanticLabel::CrossLayout => 0,
            SemanticLabel::HorizontalTitle => 1,
            SemanticLabel::VerticalTitle => 1,
            SemanticLabel::Vision => 2,
            SemanticLabel::Regular => 3,
        }
    }
}
