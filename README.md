# xycut-plus-plus

A high-performance Rust implementation of the XY-Cut++ algorithm for reading order detection in document layout analysis.

## Features

- **Pre-mask Processing**: Automatically separates titles, figures, and tables from regular text
- **Recursive Segmentation**: Uses projection histograms to find optimal cuts
- **Cross-Modal Matching**: Intelligently merges masked elements back into reading order
- **Generic API**: Works with any bounding box type via the `BoundingBox` trait
- **Column-Aware**: Handles multi-column layouts with spanning elements
- **High Performance**: Achieves 98.8% BLEU score at 514 FPS (based on original paper)

## Quick Example

```rust
use xycut_plus_plus::{XYCut, XYCutConfig, BoundingBox};

// Define your bounding box type
#[derive(Clone)]
struct MyBox {
    x1: f32, y1: f32, x2: f32, y2: f32,
    id: usize,
    is_title: bool,
}

// Implement the BoundingBox trait
impl BoundingBox for MyBox {
    fn id(&self) -> usize { self.id }
    fn bounds(&self) -> (f32, f32, f32, f32) {
        (self.x1, self.y1, self.x2, self.y2)
    }
    fn should_mask(&self) -> bool { self.is_title }
}

// Create the algorithm with default config
let xycut = XYCut::new(XYCutConfig::default());

// Compute reading order
let boxes = vec![/* your boxes */];
let order = xycut.compute_order(&boxes, 0.0, 0.0, 612.0, 792.0);

// `order` is now a Vec<usize> of IDs in reading order
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
xycut-plus-plus = "0.1.0"
```

Or install from git:

```toml
[dependencies]
xycut-plus-plus = { git = "https://github.com/home-still/xycut-plus-plus" }
```

## What is XY-Cut++?

XY-Cut++ is an enhanced version of the classic XY-Cut algorithm for determining reading order in document layouts. It addresses the limitations of traditional approaches by:

1. **Pre-masking** titles, figures, and tables to improve column detection
2. **Recursively cutting** the document using projection histograms to find natural gaps
3. **Cross-modal matching** to reintegrate masked elements at their correct positions

This makes it particularly effective for complex multi-column documents like academic papers.

## API

### Core Types

#### `XYCut`

The main algorithm struct.

```rust
pub struct XYCut {
    config: XYCutConfig,
}

impl XYCut {
    pub fn new(config: XYCutConfig) -> Self;

    pub fn compute_order<T: BoundingBox>(
        &self,
        elements: &[T],
        x_min: f32,
        y_min: f32,
        x_max: f32,
        y_max: f32,
    ) -> Vec<usize>;
}
```

#### `XYCutConfig`

Configuration parameters for the algorithm.

```rust
pub struct XYCutConfig {
    /// Minimum gap width in projection histogram (default: 7.0)
    pub min_gap: f32,

    /// Tolerance for considering elements in same row (default: 5.0)
    pub same_row_tolerance: f32,
}

impl Default for XYCutConfig {
    fn default() -> Self {
        Self {
            min_gap: 7.0,
            same_row_tolerance: 5.0,
        }
    }
}
```

#### `BoundingBox` Trait

Your bounding box type must implement this trait:

```rust
pub trait BoundingBox: Clone {
    /// Unique identifier for this element
    fn id(&self) -> usize;

    /// Bounding box coordinates (x1, y1, x2, y2) in any coordinate system
    fn bounds(&self) -> (f32, f32, f32, f32);

    /// Whether this element should be masked during initial processing
    /// Typically true for titles, figures, tables
    fn should_mask(&self) -> bool;

    /// Center point (default implementation provided)
    fn center(&self) -> (f32, f32) {
        let (x1, y1, x2, y2) = self.bounds();
        ((x1 + x2) / 2.0, (y1 + y2) / 2.0)
    }

    /// Intersection over Union with another box (default implementation provided)
    fn iou(&self, other: &Self) -> f32 { /* ... */ }
}
```

## Algorithm Overview

The XY-Cut++ algorithm works in three phases:

### Phase 1: Pre-mask Processing

Elements marked with `should_mask() == true` are separated from regular text. Additionally, elements wider than 70% of the page width (like spanning titles) are automatically masked. This prevents them from interfering with column detection.

### Phase 2: Recursive Cutting

The algorithm recursively segments the document:

1. **Try vertical cuts first** for large groups (>10 elements) to detect columns
2. **Build projection histograms** to find gaps between elements
3. **Split at the largest gap** meeting the minimum threshold
4. **Fall back to horizontal cuts** for reading flow within columns
5. **Sort remaining elements** by position when no more cuts are found

### Phase 3: Cross-Modal Matching

Masked elements are reintegrated into the reading order:

1. **Sort masked elements** by position (top-to-bottom, left-to-right)
2. **For each masked element**:
   - Calculate IoU with regular elements
   - If IoU > 0, insert after the overlapping element
   - If no overlap:
     - **Spanning elements** (>60% page width): Insert by y-position only
     - **Column elements**: Insert using left-edge distance (100px tolerance)

This produces correct reading order even for complex multi-column layouts with figures, tables, and spanning titles.

## Performance

Based on the original XY-Cut++ paper:

- **BLEU Score**: 98.8% (near-perfect reading order accuracy)
- **Speed**: 514 FPS on standard documents
- **Complexity**: O(n log n) for n elements

This Rust implementation maintains similar performance characteristics with the added benefits of memory safety and zero-cost abstractions.

## Use Cases

- **PDF Processing**: Extract text in correct reading order
- **Document Understanding**: Pre-process documents for LLMs
- **OCR Post-Processing**: Order detected text regions
- **Layout Analysis**: Understand document structure
- **Accessibility**: Generate proper reading order for screen readers

## Implementation Notes

### Column Detection

The algorithm uses **left edge distance** (100px tolerance) rather than center distance for column detection. This handles cases where boxes in the same column have different widths (e.g., narrow section headers vs. wide paragraphs).

### Spanning Elements

Elements that span multiple columns (like page titles) are detected by width (>60% page width) and inserted based on y-position only, avoiding false column matches.

### Element Masking

Two types of masking occur:
1. **Explicit masking**: Elements with `should_mask() == true`
2. **Implicit masking**: Elements wider than 70% page width

Both help the recursive cutting algorithm find column gaps.

## References

- **Paper**: "XY-Cut++ for Document Layout Analysis" (2023)
- **Original XY-Cut**: Nagy & Seth (1984)

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## Acknowledgments

This implementation is based on the XY-Cut++ algorithm described in the 2023 paper, achieving state-of-the-art performance in reading order detection.
