//! # XY-Cut++
//!
//! A high-performance reading order detection algorithm for document layout analysis
//! https://arxiv.org/pdf/2504.10258
//! Original Authors:
//! Shuai Liu, shuai liu@tju.edu.cn
//! Youmeng Li*, liyoumeng@tju.edu.cn
//! Jizeng Wei, weijizeng@tju.edu.cn

pub mod core;
pub mod histogram;
pub mod matching;
pub mod traits;
pub mod utils;

pub use core::{XYCutPlusPlus, XYCutConfig};
pub use traits::BoundingBox;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // TODO: Add real tests
    }
}
