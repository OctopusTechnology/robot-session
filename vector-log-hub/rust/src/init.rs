//! Tracing initialization functions
//!
//! This module provides various initialization functions for setting up tracing
//! with the Vector layer for different use cases.

use crate::layer::VectorLayer;
use std::sync::Once;
use tracing_subscriber::prelude::*;

/// Initialize tracing with Vector layer using default registry as specified in log-hub.md
///
/// # Arguments
/// * `service_name` - Name of the service
/// * `vector_addr` - Vector TCP socket address (default: "localhost:9000")
///
/// # Example
/// ```rust
/// use tracing_vector::init_tracing_with_default_registry;
///
/// init_tracing_with_default_registry("my-service", "localhost:9000");
/// tracing::info!("Application started");
/// ```
pub fn init_tracing_with_default_registry(service_name: &str, vector_addr: &str) {
    let vector_layer = VectorLayer::new(service_name, vector_addr);
    tracing_subscriber::registry().with(vector_layer).init();
}

/// Initialize tracing with Vector layer using a provided registry
///
/// This is the main initialization function that sets up tracing with Vector layer
/// using a provided subscriber registry for maximum flexibility.
///
/// # Arguments
/// * `registry` - The tracing subscriber registry to use
/// * `service_name` - Name of the service
/// * `vector_addr` - Vector TCP socket address (default: "localhost:9000")
///
/// # Example
/// ```rust
/// use tracing_vector::{init_tracing, VectorLayer};
/// use tracing_subscriber::Registry;
///
/// let registry = Registry::default();
/// init_tracing(registry, "my-service", "localhost:9000");
/// tracing::info!("Application started");
/// ```
pub fn init_tracing<S>(registry: S, service_name: &str, vector_addr: &str)
where
    S: tracing::Subscriber + Send + Sync + 'static,
{
    let vector_layer = VectorLayer::new(service_name, vector_addr);
    registry.with(vector_layer).init();
}

/// Initialize tracing for testing purposes (only once)
///
/// This function is specifically designed for testing and will attempt to connect
/// to Vector but won't fail if the connection is not available. It uses `std::sync::Once`
/// to ensure initialization only happens once, even if called multiple times.
///
/// # Arguments
/// * `service_name` - Name of the service
/// * `vector_addr` - Vector TCP socket address (default: "localhost:9000")
///
/// # Example
/// ```rust
/// use tracing_vector::init_test_tracing;
///
/// init_test_tracing("test-service", "localhost:9000");
/// tracing::info!("Test started");
/// ```
pub fn init_test_tracing(service_name: &str, vector_addr: &str) {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        init_tracing_with_default_registry(service_name, vector_addr);
    });
}
