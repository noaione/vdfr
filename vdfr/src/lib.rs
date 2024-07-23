pub mod common;

#[cfg(feature = "legacy")]
pub mod legacy_parser;
pub mod parser;

pub use common::*;

// Re-export serde_json, if feature serde is enabled
#[cfg(feature = "serde")]
pub use serde_json;
