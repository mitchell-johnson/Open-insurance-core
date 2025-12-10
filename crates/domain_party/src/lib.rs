//! Party Management Domain
//!
//! This crate manages all party (customer, agent, beneficiary) data
//! including KYC (Know Your Customer) processes.

pub mod party;
pub mod address;
pub mod kyc;
pub mod agent;
pub mod error;

pub use party::{Party, PartyType, Individual, Corporate};
pub use address::Address;
pub use kyc::{KycStatus, KycDocument};
pub use agent::Agent;
pub use error::PartyError;
