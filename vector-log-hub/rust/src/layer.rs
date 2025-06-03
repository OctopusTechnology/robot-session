//! Vector layer implementation for tracing
//!
//! This module contains the VectorLayer implementation as specified in log-hub.md section 5.3.1.
//! The layer sends tracing events to Vector via TCP socket using the standard JSON log format.

use serde_json::json;
use std::cell::RefCell;
use std::io::Write;
use std::net::TcpStream;
use tracing::Subscriber;
use tracing_core::Event;
use tracing_subscriber::Layer;

use crate::visitor::JsonVisitor;

thread_local! {
    static CONNECTION: RefCell<Option<TcpStream>> = const { RefCell::new(None) };
}

/// VectorLayer implementation as specified in log-hub.md section 5.3.1
///
/// This layer sends tracing events to Vector via TCP socket using the standard
/// JSON log format defined in the documentation.
pub struct VectorLayer {
    service_name: String,
    addr: String,
}

impl VectorLayer {
    /// Create a new VectorLayer
    ///
    /// # Arguments
    /// * `service_name` - Name of the service for log identification
    /// * `addr` - Vector TCP socket address (e.g., "localhost:9000")
    pub fn new(service_name: &str, addr: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
            addr: addr.to_string(),
        }
    }

    /// Execute operation with connection, creating it if needed
    fn with_connection<F, R>(&self, f: F) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce(&mut TcpStream) -> Result<R, Box<dyn std::error::Error + Send + Sync>>,
    {
        CONNECTION.with(|conn| {
            let mut conn = conn.borrow_mut();
            if conn.is_none() {
                *conn = Some(TcpStream::connect(&self.addr)?);
            }
            if let Some(ref mut stream) = *conn {
                f(stream)
            } else {
                Err("Failed to establish connection".into())
            }
        })
    }

    /// Send log entry to Vector via TCP socket
    fn send_to_vector(
        &self,
        log_entry: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.with_connection(|stream| {
            let log_json = serde_json::to_string(log_entry)?;
            stream.write_all(log_json.as_bytes())?;
            stream.write_all(b"\n")?; // Vector expects newline-delimited JSON
            stream.flush()?;
            Ok(())
        })
    }
}

impl<S> Layer<S> for VectorLayer
where
    S: Subscriber,
{
    /// Handle tracing events and send them to Vector
    ///
    /// Converts tracing events to the standard JSON log format specified in log-hub.md:
    /// ```json
    /// {
    ///   "timestamp": "2023-09-28T15:04:05Z",
    ///   "level": "info",
    ///   "message": "操作完成",
    ///   "service": "服务名称",
    ///   "context": {
    ///     "operation_id": "abc123",
    ///     "duration_ms": 42
    ///   }
    /// }
    /// ```
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let metadata = event.metadata();

        // Extract event data and create JSON log entry
        let mut visitor = JsonVisitor::new();
        event.record(&mut visitor);

        let log_entry = json!({
            "timestamp": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.9fZ").to_string(),
            "level": metadata.level().to_string().to_lowercase(),
            "message": visitor.message().unwrap_or_default(),
            "service": self.service_name,
            "context": visitor.context()
        });

        if let Err(e) = self.send_to_vector(&log_entry) {
            eprintln!("Failed to send log to Vector: {}", e);
        }
    }
}
