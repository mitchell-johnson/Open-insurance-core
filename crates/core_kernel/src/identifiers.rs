//! Strongly-typed identifiers for domain entities
//!
//! Using newtype wrappers around UUIDs provides type safety and prevents
//! accidental mixing of different identifier types.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Creates a new random identifier
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Creates a new time-ordered identifier (v7)
            pub fn new_v7() -> Self {
                Self(Uuid::now_v7())
            }

            /// Creates from an existing UUID
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Returns the underlying UUID
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            /// Returns the identifier prefix for display
            pub fn prefix() -> &'static str {
                $prefix
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}-{}", $prefix, self.0)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                // Strip prefix if present
                let uuid_str = s.strip_prefix(concat!($prefix, "-")).unwrap_or(s);
                Ok(Self(Uuid::parse_str(uuid_str)?))
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Uuid {
                id.0
            }
        }
    };
}

// Policy domain identifiers
define_id!(PolicyId, "POL");
define_id!(PolicyVersionId, "POLV");
define_id!(CoverageId, "COV");
define_id!(RiskObjectId, "RISK");
define_id!(EndorsementId, "END");

// Claims domain identifiers
define_id!(ClaimId, "CLM");
define_id!(ClaimLineId, "CLML");
define_id!(ReserveId, "RES");
define_id!(PaymentId, "PAY");

// Party domain identifiers
define_id!(PartyId, "PTY");
define_id!(AddressId, "ADDR");
define_id!(ContactId, "CNT");
define_id!(AgentId, "AGT");

// Billing domain identifiers
define_id!(AccountId, "ACC");
define_id!(JournalEntryId, "JNL");
define_id!(PostingId, "PST");
define_id!(InvoiceId, "INV");

// Fund domain identifiers
define_id!(FundId, "FND");
define_id!(UnitHoldingId, "UNT");
define_id!(NavId, "NAV");

// Generic identifiers
define_id!(VersionId, "VER");
define_id!(TransactionId, "TXN");
define_id!(AuditEventId, "AUD");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_id_display() {
        let id = PolicyId::new();
        let display = id.to_string();
        assert!(display.starts_with("POL-"));
    }

    #[test]
    fn test_id_parsing() {
        let original = PolicyId::new();
        let parsed: PolicyId = original.to_string().parse().unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_uuid_conversion() {
        let uuid = Uuid::new_v4();
        let policy_id = PolicyId::from(uuid);
        let back: Uuid = policy_id.into();
        assert_eq!(uuid, back);
    }
}
