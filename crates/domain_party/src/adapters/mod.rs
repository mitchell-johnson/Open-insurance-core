//! External Adapters for Party Domain
//!
//! This module provides adapter implementations for connecting to external
//! party/customer management systems. These adapters implement the `PartyPort`
//! trait, allowing seamless swapping between internal database storage and
//! external systems of record.
//!
//! # Available Adapters
//!
//! - **ExternalCrmAdapter**: Connects to external CRM systems via REST API
//! - **MockPartyPort**: In-memory mock for testing (re-exported from ports module)
//!
//! # Usage
//!
//! Configure the appropriate adapter at application startup based on environment:
//!
//! ```rust,ignore
//! use domain_party::adapters::{ExternalCrmAdapter, ExternalCrmConfig};
//! use domain_party::PartyPort;
//! use std::sync::Arc;
//!
//! let config = ExternalCrmConfig {
//!     base_url: "https://crm.example.com/api".to_string(),
//!     api_key: "secret".to_string(),
//!     timeout_secs: 30,
//!     retry_attempts: 3,
//! };
//!
//! let adapter = ExternalCrmAdapter::new(config);
//! let port: Arc<dyn PartyPort> = Arc::new(adapter);
//! ```

pub mod external_crm;

pub use external_crm::{ExternalCrmAdapter, ExternalCrmConfig};
