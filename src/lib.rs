//! rust-to-mojo: an honest, report-driven Rust -> Mojo source translator.
//!
//! Pipeline: `syn` parse -> crate-wide symbol pass -> lowering to a small Mojo
//! IR (`ir`) -> pretty printer (`printer`). Anything that cannot be lowered
//! faithfully becomes a `TODO(rust-to-mojo)` stub carrying the original Rust,
//! and is counted in the coverage report (`report`).

pub mod driver;
pub mod hints;
pub mod ir;
pub mod lower;
pub mod printer;
pub mod report;
pub mod symbols;

/// The Mojo language version the emitted code targets.
pub const MOJO_TARGET: &str = "Mojo 1.0 (Modular 26.5, Aug 2026): def-only functions, comptime, Deinitable, Array, std.* imports";

pub use driver::{translate_path, translate_source, write_output, Output};
pub use lower::Options;
