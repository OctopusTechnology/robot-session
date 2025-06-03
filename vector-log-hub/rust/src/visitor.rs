//! JSON visitor implementation for tracing events
//!
//! This module contains the JsonVisitor that extracts fields from tracing events
//! and converts them into JSON format for Vector.

use serde_json::json;

/// Visitor to extract fields from tracing events into JSON format
pub struct JsonVisitor {
    message: Option<String>,
    context: serde_json::Map<String, serde_json::Value>,
}

impl JsonVisitor {
    pub fn new() -> Self {
        Self {
            message: None,
            context: serde_json::Map::new(),
        }
    }

    pub fn message(&self) -> Option<String> {
        self.message.clone()
    }

    pub fn context(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.context
    }
}

impl tracing::field::Visit for JsonVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let value_str = format!("{:?}", value);
        if field.name() == "message" {
            self.message = Some(value_str);
        } else {
            self.context
                .insert(field.name().to_string(), json!(value_str));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.context.insert(field.name().to_string(), json!(value));
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.context.insert(field.name().to_string(), json!(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.context.insert(field.name().to_string(), json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.context.insert(field.name().to_string(), json!(value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.context.insert(field.name().to_string(), json!(value));
    }
}
