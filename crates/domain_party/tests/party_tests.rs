//! Comprehensive tests for domain_party

use chrono::{NaiveDate, Utc, Days};
use rust_decimal_macros::dec;

use core_kernel::{PartyId, AgentId};

use domain_party::party::{Party, PartyType, Individual, Corporate, Gender};
use domain_party::address::{Address, AddressType};
use domain_party::kyc::{KycDocument, KycStatus, DocumentType};
use domain_party::agent::{Agent, AgentStatus};

// ============================================================================
// Party Tests
// ============================================================================

mod party_tests {
    use super::*;

    fn create_test_individual() -> Individual {
        Individual {
            first_name: "John".to_string(),
            middle_name: Some("Robert".to_string()),
            last_name: "Doe".to_string(),
            date_of_birth: NaiveDate::from_ymd_opt(1985, 6, 15).unwrap(),
            gender: Some(Gender::Male),
            nationality: Some("US".to_string()),
            tax_id: Some("123-45-6789".to_string()),
            occupation: Some("Engineer".to_string()),
        }
    }

    fn create_test_corporate() -> Corporate {
        Corporate {
            company_name: "Acme Corporation".to_string(),
            registration_number: Some("REG-123456".to_string()),
            tax_id: Some("12-3456789".to_string()),
            industry: Some("Technology".to_string()),
            incorporation_date: Some(NaiveDate::from_ymd_opt(2010, 1, 1).unwrap()),
            incorporation_country: Some("US".to_string()),
        }
    }

    #[test]
    fn test_party_new_individual() {
        let individual = create_test_individual();
        let party = Party::new_individual(individual);

        assert_eq!(party.party_type, PartyType::Individual);
        assert!(party.individual.is_some());
        assert!(party.corporate.is_none());
        assert!(party.is_active);
        assert_eq!(party.kyc_status, KycStatus::Pending);
    }

    #[test]
    fn test_party_new_corporate() {
        let corporate = create_test_corporate();
        let party = Party::new_corporate(corporate);

        assert_eq!(party.party_type, PartyType::Corporate);
        assert!(party.corporate.is_some());
        assert!(party.individual.is_none());
    }

    #[test]
    fn test_party_display_name_individual() {
        let individual = create_test_individual();
        let party = Party::new_individual(individual);

        assert_eq!(party.display_name(), "John Doe");
    }

    #[test]
    fn test_party_display_name_corporate() {
        let corporate = create_test_corporate();
        let party = Party::new_corporate(corporate);

        assert_eq!(party.display_name(), "Acme Corporation");
    }

    #[test]
    fn test_party_display_name_unknown() {
        // Test agent type which returns "Unknown"
        let mut party = Party::new_individual(create_test_individual());
        party.party_type = PartyType::Agent;
        party.individual = None;

        assert_eq!(party.display_name(), "Unknown");
    }

    #[test]
    fn test_party_add_address() {
        let mut party = Party::new_individual(create_test_individual());
        let address = Address::new(
            AddressType::Residential,
            "123 Main St",
            "New York",
            "10001",
            "USA",
        );

        party.add_address(address);

        assert_eq!(party.addresses.len(), 1);
    }

    #[test]
    fn test_all_party_types() {
        let types = vec![
            PartyType::Individual,
            PartyType::Corporate,
            PartyType::Agent,
            PartyType::Broker,
        ];

        for party_type in types {
            let json = serde_json::to_string(&party_type).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_all_genders() {
        let genders = vec![
            Gender::Male,
            Gender::Female,
            Gender::Other,
        ];

        for gender in genders {
            let json = serde_json::to_string(&gender).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_individual_serialization() {
        let individual = create_test_individual();
        let json = serde_json::to_string(&individual).unwrap();
        let deserialized: Individual = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.first_name, "John");
        assert_eq!(deserialized.last_name, "Doe");
    }

    #[test]
    fn test_corporate_serialization() {
        let corporate = create_test_corporate();
        let json = serde_json::to_string(&corporate).unwrap();
        let deserialized: Corporate = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.company_name, "Acme Corporation");
    }
}

// ============================================================================
// Address Tests
// ============================================================================

mod address_tests {
    use super::*;

    #[test]
    fn test_address_new() {
        let address = Address::new(
            AddressType::Residential,
            "123 Main St",
            "New York",
            "10001",
            "USA",
        );

        assert_eq!(address.address_type, AddressType::Residential);
        assert_eq!(address.line1, "123 Main St");
        assert_eq!(address.city, "New York");
        assert_eq!(address.postal_code, "10001");
        assert_eq!(address.country, "USA");
        assert!(address.line2.is_none());
        assert!(address.state.is_none());
        assert!(!address.is_primary);
    }

    #[test]
    fn test_address_format_simple() {
        let address = Address::new(
            AddressType::Residential,
            "123 Main St",
            "New York",
            "10001",
            "USA",
        );

        let formatted = address.format();

        assert!(formatted.contains("123 Main St"));
        assert!(formatted.contains("New York 10001"));
        assert!(formatted.contains("USA"));
    }

    #[test]
    fn test_address_format_with_line2() {
        let mut address = Address::new(
            AddressType::Residential,
            "123 Main St",
            "New York",
            "10001",
            "USA",
        );
        address.line2 = Some("Apt 4B".to_string());

        let formatted = address.format();

        assert!(formatted.contains("Apt 4B"));
    }

    #[test]
    fn test_address_format_with_state() {
        let mut address = Address::new(
            AddressType::Residential,
            "123 Main St",
            "New York",
            "10001",
            "USA",
        );
        address.state = Some("NY".to_string());

        let formatted = address.format();

        assert!(formatted.contains("New York, NY 10001"));
    }

    #[test]
    fn test_all_address_types() {
        let types = vec![
            AddressType::Residential,
            AddressType::Mailing,
            AddressType::Business,
            AddressType::Billing,
        ];

        for address_type in types {
            let json = serde_json::to_string(&address_type).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_address_serialization() {
        let address = Address::new(
            AddressType::Business,
            "456 Corporate Ave",
            "Chicago",
            "60601",
            "USA",
        );

        let json = serde_json::to_string(&address).unwrap();
        let deserialized: Address = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.city, "Chicago");
    }
}

// ============================================================================
// KYC Tests
// ============================================================================

mod kyc_tests {
    use super::*;

    #[test]
    fn test_kyc_document_new() {
        let party_id = PartyId::new_v7();
        let doc = KycDocument::new(party_id, DocumentType::Passport);

        assert_eq!(doc.party_id, party_id);
        assert_eq!(doc.document_type, DocumentType::Passport);
        assert!(!doc.verified);
        assert!(doc.verified_at.is_none());
        assert!(doc.verified_by.is_none());
    }

    #[test]
    fn test_kyc_document_verify() {
        let party_id = PartyId::new_v7();
        let mut doc = KycDocument::new(party_id, DocumentType::Passport);

        doc.verify("verifier@example.com");

        assert!(doc.verified);
        assert!(doc.verified_at.is_some());
        assert_eq!(doc.verified_by, Some("verifier@example.com".to_string()));
    }

    #[test]
    fn test_kyc_document_is_expired_not_expired() {
        let party_id = PartyId::new_v7();
        let mut doc = KycDocument::new(party_id, DocumentType::Passport);
        doc.expiry_date = Some(Utc::now().date_naive() + Days::new(365));

        assert!(!doc.is_expired());
    }

    #[test]
    fn test_kyc_document_is_expired_expired() {
        let party_id = PartyId::new_v7();
        let mut doc = KycDocument::new(party_id, DocumentType::Passport);
        doc.expiry_date = Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap());

        assert!(doc.is_expired());
    }

    #[test]
    fn test_kyc_document_is_expired_no_expiry() {
        let party_id = PartyId::new_v7();
        let doc = KycDocument::new(party_id, DocumentType::Passport);

        assert!(!doc.is_expired()); // No expiry means not expired
    }

    #[test]
    fn test_all_kyc_statuses() {
        let statuses = vec![
            KycStatus::Pending,
            KycStatus::InProgress,
            KycStatus::Verified,
            KycStatus::Failed,
            KycStatus::Expired,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_all_document_types() {
        let types = vec![
            DocumentType::Passport,
            DocumentType::DriversLicense,
            DocumentType::NationalId,
            DocumentType::ProofOfAddress,
            DocumentType::TaxReturn,
            DocumentType::BankStatement,
            DocumentType::Other("Custom Document".to_string()),
        ];

        for doc_type in types {
            let json = serde_json::to_string(&doc_type).unwrap();
            assert!(!json.is_empty());
        }
    }
}

// ============================================================================
// Agent Tests
// ============================================================================

mod agent_tests {
    use super::*;

    #[test]
    fn test_agent_new() {
        let party_id = PartyId::new_v7();
        let agent = Agent::new(party_id, "AGT001");

        assert_eq!(agent.party_id, party_id);
        assert_eq!(agent.agent_code, "AGT001");
        assert_eq!(agent.status, AgentStatus::Active);
        assert!(agent.license_number.is_none());
        assert!(agent.license_expiry.is_none());
        assert!(agent.default_commission_rate.is_none());
    }

    #[test]
    fn test_agent_is_licensed_no_license_required() {
        let agent = Agent::new(PartyId::new_v7(), "AGT001");

        // No license number and no expiry means no license required
        assert!(agent.is_licensed());
    }

    #[test]
    fn test_agent_is_licensed_valid_license() {
        let mut agent = Agent::new(PartyId::new_v7(), "AGT001");
        agent.license_number = Some("LIC-12345".to_string());
        agent.license_expiry = Some(Utc::now().date_naive() + Days::new(365));

        assert!(agent.is_licensed());
    }

    #[test]
    fn test_agent_is_licensed_expired_license() {
        let mut agent = Agent::new(PartyId::new_v7(), "AGT001");
        agent.license_number = Some("LIC-12345".to_string());
        agent.license_expiry = Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap());

        assert!(!agent.is_licensed());
    }

    #[test]
    fn test_all_agent_statuses() {
        let statuses = vec![
            AgentStatus::Active,
            AgentStatus::Inactive,
            AgentStatus::Suspended,
            AgentStatus::Terminated,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_agent_serialization() {
        let agent = Agent::new(PartyId::new_v7(), "AGT001");

        let json = serde_json::to_string(&agent).unwrap();
        let deserialized: Agent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.agent_code, "AGT001");
    }

    #[test]
    fn test_agent_with_commission_rate() {
        let mut agent = Agent::new(PartyId::new_v7(), "AGT001");
        agent.default_commission_rate = Some(dec!(10.5));

        assert_eq!(agent.default_commission_rate, Some(dec!(10.5)));
    }

    #[test]
    fn test_agent_with_manager() {
        let mut agent = Agent::new(PartyId::new_v7(), "AGT001");
        let manager_id = AgentId::new_v7();
        agent.manager_id = Some(manager_id);

        assert_eq!(agent.manager_id, Some(manager_id));
    }

    #[test]
    fn test_agent_with_territory() {
        let mut agent = Agent::new(PartyId::new_v7(), "AGT001");
        agent.territory = Some("Northeast".to_string());

        assert_eq!(agent.territory, Some("Northeast".to_string()));
    }
}
