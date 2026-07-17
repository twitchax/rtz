//! The `rtz-core` crate.  Abstracts away core functionality, and functionality for build scripts.

#![warn(rustdoc::broken_intra_doc_links, rust_2018_idioms, clippy::all, missing_docs)]
#![allow(incomplete_features)]
// `coverage(off)` is nightly-only; the feature (and the attrs below) activate only under
// `cargo llvm-cov` on nightly, which sets `coverage_nightly`. On stable they are inert, so the
// crate still builds and publishes on stable.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod base;
pub mod geo;
