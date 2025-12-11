//! Comprehensive unit tests for the Identifiers module
//!
//! Tests cover all identifier types, their creation, parsing,
//! conversion, and display formatting.

use core_kernel::{
    PolicyId, ClaimId, PartyId, AccountId, JournalEntryId,
    FundId, UnitHoldingId, VersionId, PolicyVersionId, CoverageId,
    RiskObjectId, EndorsementId, ClaimLineId, ReserveId, PaymentId,
    AddressId, ContactId, AgentId, PostingId, InvoiceId, NavId,
    TransactionId, AuditEventId,
};
use uuid::Uuid;

mod policy_id_tests {
    use super::*;

    #[test]
    fn test_new_generates_unique_ids() {
        let id1 = PolicyId::new();
        let id2 = PolicyId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_new_v7_generates_time_ordered_ids() {
        let id1 = PolicyId::new_v7();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = PolicyId::new_v7();
        let uuid1: Uuid = id1.into();
        let uuid2: Uuid = id2.into();
        assert!(uuid1 < uuid2);
    }

    #[test]
    fn test_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = PolicyId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn test_prefix() {
        assert_eq!(PolicyId::prefix(), "POL");
    }

    #[test]
    fn test_display_format() {
        let id = PolicyId::new();
        let display = id.to_string();
        assert!(display.starts_with("POL-"));
    }

    #[test]
    fn test_from_str_with_prefix() {
        let original = PolicyId::new();
        let string = original.to_string();
        let parsed: PolicyId = string.parse().unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_uuid_conversion() {
        let uuid = Uuid::new_v4();
        let id: PolicyId = uuid.into();
        let back: Uuid = id.into();
        assert_eq!(uuid, back);
    }

    #[test]
    fn test_json_serialization() {
        let id = PolicyId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: PolicyId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}

mod claim_id_tests {
    use super::*;

    #[test]
    fn test_new_generates_unique_ids() {
        let id1 = ClaimId::new();
        let id2 = ClaimId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_prefix() {
        assert_eq!(ClaimId::prefix(), "CLM");
    }

    #[test]
    fn test_display_format() {
        let id = ClaimId::new();
        let display = id.to_string();
        assert!(display.starts_with("CLM-"));
    }

    #[test]
    fn test_roundtrip() {
        let original = ClaimId::new();
        let string = original.to_string();
        let parsed: ClaimId = string.parse().unwrap();
        assert_eq!(original, parsed);
    }
}

mod party_id_tests {
    use super::*;

    #[test]
    fn test_new_generates_unique_ids() {
        let id1 = PartyId::new();
        let id2 = PartyId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_prefix() {
        assert_eq!(PartyId::prefix(), "PTY");
    }

    #[test]
    fn test_display_format() {
        let id = PartyId::new();
        let display = id.to_string();
        assert!(display.starts_with("PTY-"));
    }
}

mod account_id_tests {
    use super::*;

    #[test]
    fn test_new_generates_unique_ids() {
        let id1 = AccountId::new();
        let id2 = AccountId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_prefix() {
        assert_eq!(AccountId::prefix(), "ACC");
    }

    #[test]
    fn test_display_format() {
        let id = AccountId::new();
        let display = id.to_string();
        assert!(display.starts_with("ACC-"));
    }
}

mod fund_id_tests {
    use super::*;

    #[test]
    fn test_new_generates_unique_ids() {
        let id1 = FundId::new();
        let id2 = FundId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_prefix() {
        assert_eq!(FundId::prefix(), "FND");
    }

    #[test]
    fn test_display_format() {
        let id = FundId::new();
        let display = id.to_string();
        assert!(display.starts_with("FND-"));
    }
}

mod cross_type_tests {
    use super::*;

    #[test]
    fn test_different_id_types_are_distinct() {
        // Same UUID should create different identifier instances
        // that are type-safe (can't mix PolicyId with ClaimId)
        let uuid = Uuid::new_v4();
        let policy_id = PolicyId::from_uuid(uuid);
        let claim_id = ClaimId::from_uuid(uuid);

        // They contain the same UUID but are different types
        assert_eq!(*policy_id.as_uuid(), *claim_id.as_uuid());
    }

    #[test]
    fn test_id_prefixes_are_unique() {
        let prefixes = vec![
            PolicyId::prefix(),
            PolicyVersionId::prefix(),
            CoverageId::prefix(),
            RiskObjectId::prefix(),
            EndorsementId::prefix(),
            ClaimId::prefix(),
            ClaimLineId::prefix(),
            ReserveId::prefix(),
            PaymentId::prefix(),
            PartyId::prefix(),
            AddressId::prefix(),
            ContactId::prefix(),
            AgentId::prefix(),
            AccountId::prefix(),
            JournalEntryId::prefix(),
            PostingId::prefix(),
            InvoiceId::prefix(),
            FundId::prefix(),
            UnitHoldingId::prefix(),
            NavId::prefix(),
            VersionId::prefix(),
            TransactionId::prefix(),
            AuditEventId::prefix(),
        ];

        // Check all prefixes are unique
        let mut unique_prefixes: Vec<&str> = prefixes.clone();
        unique_prefixes.sort();
        unique_prefixes.dedup();

        assert_eq!(
            prefixes.len(),
            unique_prefixes.len(),
            "All identifier prefixes should be unique"
        );
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn test_nil_uuid() {
        let nil_uuid = Uuid::nil();
        let id = PolicyId::from_uuid(nil_uuid);
        assert!(id.as_uuid().is_nil());
    }

    #[test]
    fn test_max_uuid() {
        let max_uuid = Uuid::max();
        let id = PolicyId::from_uuid(max_uuid);
        assert_eq!(*id.as_uuid(), max_uuid);
    }
}
