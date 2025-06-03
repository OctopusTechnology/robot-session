//! # Tracing Vector
//!
//! A tracing layer implementation for sending logs to Vector log aggregator via TCP socket.
//! This implementation follows the specifications in log-hub.md section 5.3.1.
//!
//! ## Usage
//!
//! ```rust
//! use tracing_vector::VectorLayer;
//! use tracing_subscriber::{Registry, prelude::*};
//!
//! // Initialize tracing with Vector layer
//! let vector_layer = VectorLayer::new("my-service", "localhost:9000");
//! Registry::default().with(vector_layer).init();
//!
//! // Use tracing macros as normal
//! tracing::info!(message = "Application started", version = "1.0.0");
//! ```

// Module declarations
mod init;
mod layer;
mod visitor;

#[cfg(test)]
mod tests;

// Re-exports for public API
pub use init::{init_test_tracing, init_tracing, init_tracing_with_default_registry};
pub use layer::VectorLayer;
