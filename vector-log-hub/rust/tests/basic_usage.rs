//! Basic usage example for tracing-vector
//!
//! This example demonstrates the basic setup and usage of the VectorLayer
//! as specified in log-hub.md section 5.3.1

use tracing::{debug, error, info, warn};
use tracing_vector::init_test_tracing;

fn main() {
    println!("Starting basic usage example...");

    // Method 1: Using the convenience function
    init_test_tracing("basic-example", "localhost:9000");

    // Basic logging
    info!(message = "Application started", version = "1.0.0");
    debug!(message = "Debug information", component = "main");
    warn!(message = "This is a warning", code = "W001");
    error!(message = "This is an error", code = "E001");

    // Structured logging with context
    info!(
        message = "User operation",
        user_id = 12345,
        operation = "login",
        ip_address = "192.168.1.100",
        session_id = "sess_abc123"
    );

    // Logging with various data types
    info!(
        message = "Database operation",
        operation = "SELECT",
        table = "users",
        rows_affected = 42,
        duration_ms = 156,
        success = true,
        query_cost = 0.025
    );

    println!("Basic usage example completed. Check Vector logs at localhost:9000");
}
