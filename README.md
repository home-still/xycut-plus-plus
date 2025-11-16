# XY-Cut++

[![Crates.io](https://img.shields.io/crates/v/xycut-plus-plus.svg)](https://crates.io/crates/xycut-plus-plus)
[![Documentation](https://docs.rs/xycut-plus-plus/badge.svg)](https://docs.rs/xycut-plus-plus)
[![License](https://img.shields.io/crates/l/xycut-plus-plus.svg)](LICENSE)

High-performance document reading order detection for complex layouts. Implements the XY-Cut++ algorithm with hierarchical mask mechanism for accurate layout ordering in multi-column documents, newspapers, and academic papers.

**Paper**: [XY-Cut++: Advanced Layout Ordering via Hierarchical Mask Mechanism](https://arxiv.org/abs/2504.10258)  
**Authors**: Shuai Liu, Youmeng Li, Jizeng Wei (Tianjin University)

## Features

- **State-of-the-art accuracy**: 98.8% BLEU score on DocBench-100 benchmark
- **Fast**: 514 FPS average (1.06× faster than geometric-only methods)
- **Zero-copy design**: Efficient memory usage with trait-based abstractions
- **Safe Rust**: 100% safe code with no `unsafe` blocks
- **Complex layout support**: Handles multi-column, nested, and cross-page elements
- **Semantic-aware**: Uses shallow semantic labels (titles, figures, tables) to improve ordering

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
xycut-plus-plus = "0.1"
```

### Basic Example

```rust
use xycut_plus_plus::{XYCutPlusPlus, XYCutConfig, BoundingBox, SemanticLabel};

// 1. Implement BoundingBox for your element type
#[derive(Clone)]
struct Element {
    id: usize,
    x1: f32, y1: f32, x2: f32, y2: f32,
    label: SemanticLabel,
}

impl BoundingBox for Element {
    fn id(&self) -> usize { self.id }
    
    fn center(&self) -> (f32, f32) {
        ((self.x1 + self.x2) / 2.0, (self.y1 + self.y2) / 2.0)
    }
    
    fn bounds(&self) -> (f32, f32, f32, f32) {
        (self.x1, self.y1, self.x2, self.y2)
    }
    
    fn iou(&self, other: &Self) -> f32 {
        // Intersection over Union implementation
        let x_overlap = (self.x2.min(other.x2) - self.x1.max(other.x1)).max(0.0);
        let y_overlap = (self.y2.min(other.y2) - self.y1.max(other.y1)).max(0.0);
        let intersection = x_overlap * y_overlap;
        let union = (self.x2 - self.x1) * (self.y2 - self.y1)
                  + (other.x2 - other.x1) * (other.y2 - other.y1)
                  - intersection;
        if union > 0.0 { intersection / union } else { 0.0 }
    }
    
    fn should_mask(&self) -> bool {
        matches!(self.label, 
            SemanticLabel::HorizontalTitle | 
            SemanticLabel::VerticalTitle | 
            SemanticLabel::Vision)
    }
    
    fn semantic_label(&self) -> SemanticLabel { self.label }
}

// 2. Create elements from your layout detection
let elements = vec![
    Element { id: 0, x1: 10.0, y1: 10.0, x2: 200.0, y2: 30.0, 
              label: SemanticLabel::HorizontalTitle },
    Element { id: 1, x1: 10.0, y1: 50.0, x2: 400.0, y2: 100.0,
              label: SemanticLabel::Regular },
    // ... more elements
];

// 3. Compute reading order
let xycut = XYCutPlusPlus::new(XYCutConfig::default());
let page_bounds = (0.0, 0.0, 800.0, 1200.0);  // (x_min, y_min, x_max, y_max)

let ordered_ids = xycut.compute_order(
    &elements,
    page_bounds.0, page_bounds.1,
    page_bounds.2, page_bounds.3
);

// ordered_ids = [0, 1, ...] in correct reading order
for id in ordered_ids {
    println!("Read element {}", id);
}
```

## Algorithm Overview

XY-Cut++ extends the classic XY-Cut algorithm with three key innovations:

1. **Pre-Mask Processing** (Equations 1-3): Identifies and temporarily masks high-dynamic-range elements (titles, figures, tables) to prevent segmentation errors

2. **Multi-Granularity Segmentation** (Equations 4-5): Adaptively switches between horizontal-first and vertical-first cutting based on content density ratio τd

3. **Cross-Modal Matching** (Equations 7-10): Reintegrates masked elements using geometry-semantic fusion with 4-component distance metric

```
┌─────────────────────────────────────────────┐
│  Layout Detection (PP-DocLayout, etc.)      │
└──────────────────┬──────────────────────────┘
                   │
        ┌──────────▼────────────────┐
        │  Pre-Mask Processing      │ (Eq 1-3)
        │  • Adaptive threshold     │
        │  • Cross-layout detection │
        └──────────┬────────────────┘
                   │
    ┌──────────────▼──────────────────┐
    │ Multi-Granularity Segmentation  │ (Eq 4-5)
    │ • Density-driven axis selection │
    │ • Recursive XY/YX-Cut           │
    └──────────────┬──────────────────┘
                   │
        ┌──────────▼────────────┐
        │ Cross-Modal Matching  │ (Eq 7-10)
        │ • Semantic filtering  │
        │ • Distance metric     │
        └──────────┬────────────┘
                   │
            ┌──────▼────────┐
            │ Reading Order │
            └───────────────┘
```

## Performance

**DocBench-100 Benchmark** (30 complex + 70 regular layouts):

| Method | Complex BLEU-4 | Regular BLEU-4 | Overall | FPS |
|--------|----------------|----------------|---------|-----|
| XY-Cut | 74.9% | 81.8% | 79.7% | 685 |
| LayoutReader | 65.6% | 84.4% | 78.8% | 17 |
| MinerU | 70.1% | 94.6% | 87.3% | 10 |
| **XY-Cut++** | **98.6%** | **98.9%** | **98.8%** | **781** |

**OmniDocBench** (larger-scale evaluation):

| Layout Type | XY-Cut++ BLEU-4 | ARD ↓ | Tau ↑ |
|-------------|-----------------|-------|-------|
| Single-column | 99.3% | 0.004 | 0.996 |
| Double-column | 95.1% | 0.027 | 0.974 |
| Three-column | 96.7% | 0.033 | 0.984 |
| Complex | 90.1% | 0.064 | 0.942 |

See [paper](https://arxiv.org/abs/2504.10258) Section 4.3 for full results.

## Configuration

Customize behavior with `XYCutConfig`:

```rust
use xycut_plus_plus::XYCutConfig;

let config = XYCutConfig {
    min_cut_threshold: 15.0,          // Minimum gap size for cuts (pixels)
    histogram_resolution_scale: 0.5,   // Histogram bins per pixel (0.5 = 1 bin per 2px)
    same_row_tolerance: 10.0,          // Y-distance tolerance for "same row" (pixels)
};

let xycut = XYCutPlusPlus::new(config);
```

**Tuning Guidelines**:
- **min_cut_threshold**: Increase (20-30) for documents with tight spacing; decrease (5-10) for loose layouts
- **histogram_resolution_scale**: Higher values (1.0) give finer granularity but slower performance
- **same_row_tolerance**: Match to your document's line spacing (typically 5-15px)

## Use Cases

**Perfect for:**
- 📄 Academic paper parsing (multi-column PDFs)
- 📰 Newspaper digitization (complex layouts)
- 📚 Book/textbook conversion (varied structures)
- 🔍 RAG preprocessing (reading order matters!)
- 🤖 LLM data preparation (structured documents)

**Integration Examples:**
- **With `pdfium-render`**: Extract pages → detect layout → order elements → OCR
- **With `tesseract`**: Pre-order regions before OCR for better context
- **With vector DBs**: Maintain document structure in embeddings

## API Documentation

Full API documentation available at [docs.rs/xycut-plus-plus](https://docs.rs/xycut-plus-plus).

**Key Types:**
- `XYCutPlusPlus` - Main algorithm struct
- `XYCutConfig` - Configuration parameters
- `BoundingBox` - Trait for layout elements (must implement)
- `SemanticLabel` - Element type classification

## Citation

If you use this implementation in research, please cite:

```bibtex
@article{liu2025xycutplusplus,
  title={XY-Cut++: Advanced Layout Ordering via Hierarchical Mask Mechanism on a Novel Benchmark},
  author={Liu, Shuai and Li, Youmeng and Wei, Jizeng},
  journal={arXiv preprint arXiv:2504.10258},
  year={2025}
}
```

## Contributing

Contributions welcome! Please:

1. **Open an issue** before major changes
2. **Follow existing code style** (run `cargo fmt`)
3. **Add tests** for new features
4. **Update documentation** as needed

Run tests and checks:
```bash
cargo test --all
cargo clippy -- -D warnings
cargo fmt --check
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Acknowledgments

Original algorithm by Shuai Liu, Youmeng Li, and Jizeng Wei at Tianjin University.  
Rust implementation maintains 100% fidelity to the published paper (arXiv:2504.10258).

---

**Links**: [Paper](https://arxiv.org/abs/2504.10258) | [Docs](https://docs.rs/xycut-plus-plus) | [Crates.io](https://crates.io/crates/xycut-plus-plus)
