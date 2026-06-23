//! # `core` — shared domain model
//!
//! The dependency rule: everything depends on `core`; `core` depends on
//! nothing inside this repo. This keeps the domain model pure and testable
//! without any network or disk.

pub mod error;
pub mod instrument;
pub mod market;
pub mod order;
pub mod primitives;
