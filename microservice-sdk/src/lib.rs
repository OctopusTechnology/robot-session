//! Microservice SDK for integrating with the Session Manager
//!
//! This SDK provides a simple API for microservices to:
//! - Register themselves with the session manager
//! - Join LiveKit rooms when requested
//! - Notify the session manager when ready

pub mod client;
pub mod errors;
pub mod models;
pub mod traits;

pub use client::{MicroserviceRunner, SessionManagerClient};
pub use errors::*;
pub use models::*;
pub use traits::*;
