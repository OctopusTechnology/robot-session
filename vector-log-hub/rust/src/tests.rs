use crate::{init_test_tracing, VectorLayer};
use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};

/// Test function with instrumentation as shown in log-hub.md
#[instrument]
fn process_request(user_id: u64) {
    info!("Processing request for user {}", user_id);

    // Simulate some processing
    std::thread::sleep(Duration::from_millis(42));

    info!(
        message = "Request processed successfully",
        operation_id = "abc123",
        duration_ms = 42
    );
}

#[instrument]
fn process_with_error(user_id: u64) -> Result<(), &'static str> {
    warn!("Starting risky operation for user {}", user_id);

    // Simulate an error condition
    if user_id == 999 {
        error!(
            message = "Critical error occurred",
            error_code = "E001",
            user_id = user_id
        );
        return Err("User not found");
    }

    info!("Operation completed successfully");
    Ok(())
}

#[test]
fn test_vector_layer_creation() {
    let layer = VectorLayer::new("test-service", "localhost:9000");
    // Note: We can't directly access private fields, but we can test that creation succeeds
    // This test mainly ensures the constructor works without panicking
    drop(layer);
}

#[test]
fn test_tracing_integration() {
    // Initialize tracing with Vector layer
    init_test_tracing("rust-test-service", "127.0.0.1:9000");

    // Test basic info logging
    info!(message = "Test application started", version = "1.0.0");

    // Test with context
    info!(
        message = "User login",
        user_id = 12345,
        session_id = "sess_abc123",
        ip_address = "192.168.1.100"
    );

    println!("✓ Basic tracing integration test completed");
}

#[test]
fn test_instrumented_function() {
    init_test_tracing("rust-test-service", "127.0.0.1:9000");

    // Test instrumented function as per log-hub.md example
    process_request(12345);

    println!("✓ Instrumented function test completed");
}

#[test]
fn test_error_handling() {
    init_test_tracing("rust-test-service", "127.0.0.1:9000");

    // Test successful operation
    match process_with_error(123) {
        Ok(_) => info!("Operation succeeded"),
        Err(e) => error!("Operation failed: {}", e),
    }

    // Test error operation
    match process_with_error(999) {
        Ok(_) => info!("Operation succeeded"),
        Err(e) => error!("Operation failed: {}", e),
    }

    println!("✓ Error handling test completed");
}

#[test]
fn test_different_log_levels() {
    init_test_tracing("rust-test-service", "127.0.0.1:9000");

    debug!(message = "Debug message", component = "test");
    info!(message = "Info message", component = "test");
    warn!(message = "Warning message", component = "test");
    error!(message = "Error message", component = "test");

    println!("✓ Different log levels test completed");
}

#[test]
fn test_structured_logging() {
    init_test_tracing("rust-test-service", "127.0.0.1:9000");

    // Test structured logging with various data types
    info!(
        message = "Database operation",
        operation = "SELECT",
        table = "users",
        rows_affected = 5,
        duration_ms = 23,
        success = true
    );

    info!(
        message = "API request",
        method = "POST",
        endpoint = "/api/users",
        status_code = 201,
        response_time_ms = 156
    );

    println!("✓ Structured logging test completed");
}

#[test]
fn test_rapid_logging() {
    init_test_tracing("rust-test-service", "127.0.0.1:9000");

    // Test Vector's buffering with rapid log generation
    for i in 0..20 {
        info!(
            message = "Rapid log message",
            sequence = i,
            batch_id = "rapid-test",
            timestamp = chrono::Utc::now().timestamp()
        );

        // Small delay to avoid overwhelming
        std::thread::sleep(Duration::from_millis(5));
    }

    println!("✓ Rapid logging test completed");
}
