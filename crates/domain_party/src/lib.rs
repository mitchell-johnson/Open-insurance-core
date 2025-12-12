//! Party Management Domain
//!
//! This crate manages all party (customer, agent, beneficiary) data
//! including KYC (Know Your Customer) processes.
//!
//! # Party Composition Model
//!
//! The party system supports complex ownership structures through composite parties:
//!
//! - **Individual**: A single natural person
//! - **Corporate**: A single legal entity
//! - **Joint**: Multiple individuals sharing ownership (e.g., married couples)
//! - **Trust**: A trust entity with trustees
//! - **Partnership**: A business partnership with partners
//!
//! This allows policies to reference a single `policyholder_id` while supporting
//! complex ownership scenarios like joint owners, trusts, and partnerships.
//!
//! # Examples
//!
//! ```rust
//! use domain_party::party::{Party, Individual, JointDetails, JointType, PartyMember};
//! use rust_decimal_macros::dec;
//! use chrono::NaiveDate;
//!
//! // Create individual parties for husband and wife
//! let husband = Party::new_individual(Individual {
//!     first_name: "John".to_string(),
//!     middle_name: None,
//!     last_name: "Smith".to_string(),
//!     date_of_birth: NaiveDate::from_ymd_opt(1980, 5, 15).unwrap(),
//!     gender: None,
//!     nationality: None,
//!     tax_id: None,
//!     occupation: None,
//! });
//!
//! let wife = Party::new_individual(Individual {
//!     first_name: "Jane".to_string(),
//!     middle_name: None,
//!     last_name: "Smith".to_string(),
//!     date_of_birth: NaiveDate::from_ymd_opt(1982, 8, 20).unwrap(),
//!     gender: None,
//!     nationality: None,
//!     tax_id: None,
//!     occupation: None,
//! });
//!
//! // Create a joint party representing both
//! let joint_party = Party::new_joint(
//!     JointDetails {
//!         display_name: "John & Jane Smith".to_string(),
//!         joint_type: JointType::JointTenants,
//!         notes: None,
//!     },
//!     vec![
//!         PartyMember::new_owner(husband.id, dec!(50)),
//!         PartyMember::new_owner(wife.id, dec!(50)),
//!     ],
//! );
//!
//! // The joint_party.id can now be used as a policy's policyholder_id
//! ```

pub mod party;
pub mod address;
pub mod kyc;
pub mod agent;
pub mod error;
pub mod validation;
pub mod ports;
pub mod adapters;

pub use party::{
    Party, PartyType, PartyComposition, Individual, Corporate, Gender,
    PartyMember, MemberRole,
    JointDetails, JointType,
    TrustDetails, TrustType,
    PartnershipDetails, PartnershipType,
    CorporateType,
};
pub use address::{Address, AddressType};
pub use kyc::{KycStatus, KycDocument};
pub use agent::Agent;
pub use error::PartyError;
pub use validation::{PartyValidator, ValidationResult};
pub use ports::{
    PartyPort, PartyPortExt, PartyQuery,
    CreatePartyRequest, CreateMemberRequest, UpdatePartyRequest,
};
#[cfg(any(test, feature = "mock"))]
pub use ports::mock::MockPartyPort;
pub use adapters::{ExternalCrmAdapter, ExternalCrmConfig};
