//! Execution context management package
//!
//! This package implements a thread-safe execution context structure for propagating properties
//! accross processes (or plugins) boundaries.
//! A [Context] is an immutable data structure that contains a bunch of properties. When writing
//! data to a context, the latter is cloned and the new property is appended to it (a kind of copy)
//! on write pattern).

// Declare package modules
pub mod constants;
pub mod context;
pub mod guard;

// Export package modules
pub use crate::context::Context;
