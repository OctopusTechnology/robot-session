//! Instrumented service example for tracing-vector
//!
//! This example demonstrates advanced usage with instrumented functions
//! as specified in log-hub.md section 5.3.1

use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};
use tracing_vector::init_test_tracing;

/// Simulated user service with instrumentation
struct UserService {
    service_name: String,
}

impl UserService {
    fn new(name: &str) -> Self {
        Self {
            service_name: name.to_string(),
        }
    }

    /// Process user request with full instrumentation
    #[instrument(skip(self), fields(service = %self.service_name))]
    async fn process_user_request(
        &self,
        user_id: u64,
        operation: &str,
    ) -> Result<String, &'static str> {
        info!(
            message = "Starting user request processing",
            user_id = user_id,
            operation = operation
        );

        // Simulate some async work
        tokio::time::sleep(Duration::from_millis(50)).await;

        match operation {
            "login" => self.handle_login(user_id).await,
            "logout" => self.handle_logout(user_id).await,
            "profile" => self.get_profile(user_id).await,
            _ => {
                error!(
                    message = "Unknown operation",
                    operation = operation,
                    user_id = user_id
                );
                Err("Unknown operation")
            }
        }
    }

    #[instrument(skip(self), fields(service = %self.service_name))]
    async fn handle_login(&self, user_id: u64) -> Result<String, &'static str> {
        debug!(message = "Validating user credentials", user_id = user_id);

        // Simulate validation
        tokio::time::sleep(Duration::from_millis(30)).await;

        if user_id == 999 {
            error!(
                message = "Login failed - user not found",
                user_id = user_id,
                error_code = "AUTH001"
            );
            return Err("User not found");
        }

        info!(
            message = "User login successful",
            user_id = user_id,
            session_duration_ms = 30
        );

        Ok(format!("session_{}", user_id))
    }

    #[instrument(skip(self), fields(service = %self.service_name))]
    async fn handle_logout(&self, user_id: u64) -> Result<String, &'static str> {
        info!(message = "Processing logout", user_id = user_id);

        // Simulate logout processing
        tokio::time::sleep(Duration::from_millis(20)).await;

        info!(
            message = "User logout successful",
            user_id = user_id,
            cleanup_duration_ms = 20
        );

        Ok("logged_out".to_string())
    }

    #[instrument(skip(self), fields(service = %self.service_name))]
    async fn get_profile(&self, user_id: u64) -> Result<String, &'static str> {
        debug!(message = "Fetching user profile", user_id = user_id);

        // Simulate database query
        tokio::time::sleep(Duration::from_millis(40)).await;

        info!(
            message = "Profile retrieved successfully",
            user_id = user_id,
            query_duration_ms = 40,
            profile_size_bytes = 1024
        );

        Ok(format!("profile_data_{}", user_id))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting instrumented service example...");

    // Initialize tracing with Vector layer
    init_test_tracing("instrumented-service", "localhost:9000");

    info!(
        message = "Service starting",
        service_type = "user_service",
        version = "2.0.0"
    );

    let user_service = UserService::new("user-management");

    // Test various operations
    let operations = vec![
        (12345, "login"),
        (12345, "profile"),
        (67890, "login"),
        (67890, "logout"),
        (999, "login"),     // This will fail
        (11111, "unknown"), // This will fail
    ];

    info!(
        message = "Service shutting down",
        total_operations = operations.len()
    );

    for (user_id, operation) in operations {
        match user_service.process_user_request(user_id, operation).await {
            Ok(result) => {
                info!(
                    message = "Operation completed successfully",
                    user_id = user_id,
                    operation = operation,
                    result = result
                );
            }
            Err(error) => {
                warn!(
                    message = "Operation failed",
                    user_id = user_id,
                    operation = operation,
                    error = error
                );
            }
        }

        // Small delay between operations
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    println!("Instrumented service example completed. Check Vector logs at localhost:9000");
    Ok(())
}
