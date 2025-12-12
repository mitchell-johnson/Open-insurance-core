//! Comprehensive tests for domain_party
//!
//! This module contains unit tests for all party domain functionality,
//! including individual, corporate, joint, trust, and partnership parties.

use chrono::{NaiveDate, Utc, Days};
use rust_decimal_macros::dec;

use core_kernel::PartyId;

use domain_party::party::{
    Party, PartyType, PartyComposition, Individual, Corporate, Gender,
    PartyMember, MemberRole,
    JointDetails, JointType,
    TrustDetails, TrustType,
    PartnershipDetails, PartnershipType,
    CorporateType,
};
use domain_party::address::{Address, AddressType};
use domain_party::kyc::{KycDocument, KycStatus, DocumentType};
use domain_party::agent::{Agent, AgentStatus};
use domain_party::validation::{PartyValidator, ValidationResult};

// ============================================================================
// Test Helpers
// ============================================================================

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
        corporate_type: Some(CorporateType::Corporation),
    }
}

fn create_test_joint_details() -> JointDetails {
    JointDetails {
        display_name: "John & Jane Doe".to_string(),
        joint_type: JointType::JointTenants,
        notes: Some("Married couple".to_string()),
    }
}

fn create_test_trust_details() -> TrustDetails {
    TrustDetails {
        trust_name: "Doe Family Trust".to_string(),
        trust_id: Some("TRUST-123".to_string()),
        established_date: Some(NaiveDate::from_ymd_opt(2020, 1, 15).unwrap()),
        trust_type: TrustType::RevocableLiving,
        is_revocable: true,
        governing_jurisdiction: Some("CA".to_string()),
    }
}

fn create_test_partnership_details() -> PartnershipDetails {
    PartnershipDetails {
        partnership_name: "Doe & Smith LLP".to_string(),
        registration_number: Some("LLP-456".to_string()),
        tax_id: Some("98-7654321".to_string()),
        partnership_type: PartnershipType::LLP,
        formation_date: Some(NaiveDate::from_ymd_opt(2018, 6, 1).unwrap()),
        formation_jurisdiction: Some("NY".to_string()),
    }
}

// ============================================================================
// Individual Party Tests
// ============================================================================

mod individual_party_tests {
    use super::*;

    #[test]
    fn test_new_individual_party() {
        let individual = create_test_individual();
        let party = Party::new_individual(individual);

        assert_eq!(party.party_type, PartyType::Individual);
        assert_eq!(party.composition, PartyComposition::Individual);
        assert!(party.individual.is_some());
        assert!(party.corporate.is_none());
        assert!(party.joint_details.is_none());
        assert!(party.trust_details.is_none());
        assert!(party.partnership_details.is_none());
        assert!(party.members.is_empty());
        assert!(party.is_active);
        assert_eq!(party.kyc_status, KycStatus::Pending);
    }

    #[test]
    fn test_individual_display_name() {
        let individual = create_test_individual();
        let party = Party::new_individual(individual);

        assert_eq!(party.display_name(), "John Doe");
    }

    #[test]
    fn test_individual_full_name() {
        let individual = create_test_individual();
        assert_eq!(individual.full_name(), "John Robert Doe");
    }

    #[test]
    fn test_individual_full_name_no_middle() {
        let mut individual = create_test_individual();
        individual.middle_name = None;
        assert_eq!(individual.full_name(), "John Doe");
    }

    #[test]
    fn test_individual_age_calculation() {
        let mut individual = create_test_individual();
        // Set DOB to exactly 40 years ago
        let today = Utc::now().date_naive();
        individual.date_of_birth = today
            .checked_sub_months(chrono::Months::new(12 * 40))
            .unwrap();

        let age = individual.age();
        assert!(age == 39 || age == 40); // Depending on exact date
    }

    #[test]
    fn test_individual_is_not_composite() {
        let party = Party::new_individual(create_test_individual());
        assert!(!party.is_composite());
    }

    #[test]
    fn test_individual_cannot_add_member() {
        let mut party = Party::new_individual(create_test_individual());
        let member = PartyMember::new_owner(PartyId::new_v7(), dec!(50));

        let result = party.add_member(member);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-composite"));
    }

    #[test]
    fn test_individual_serialization() {
        let individual = create_test_individual();
        let party = Party::new_individual(individual);

        let json = serde_json::to_string(&party).unwrap();
        let deserialized: Party = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.party_type, PartyType::Individual);
        assert_eq!(
            deserialized.individual.unwrap().first_name,
            "John"
        );
    }
}

// ============================================================================
// Corporate Party Tests
// ============================================================================

mod corporate_party_tests {
    use super::*;

    #[test]
    fn test_new_corporate_party() {
        let corporate = create_test_corporate();
        let party = Party::new_corporate(corporate);

        assert_eq!(party.party_type, PartyType::Corporate);
        assert_eq!(party.composition, PartyComposition::Corporate);
        assert!(party.corporate.is_some());
        assert!(party.individual.is_none());
    }

    #[test]
    fn test_corporate_display_name() {
        let corporate = create_test_corporate();
        let party = Party::new_corporate(corporate);

        assert_eq!(party.display_name(), "Acme Corporation");
    }

    #[test]
    fn test_corporate_is_not_composite() {
        let party = Party::new_corporate(create_test_corporate());
        assert!(!party.is_composite());
    }

    #[test]
    fn test_corporate_types() {
        let types = vec![
            CorporateType::LLC,
            CorporateType::Corporation,
            CorporateType::SoleProprietorship,
            CorporateType::NonProfit,
            CorporateType::Government,
            CorporateType::Other("Custom".to_string()),
        ];

        for corp_type in types {
            let json = serde_json::to_string(&corp_type).unwrap();
            assert!(!json.is_empty());
        }
    }
}

// ============================================================================
// Joint Party Tests
// ============================================================================

mod joint_party_tests {
    use super::*;

    #[test]
    fn test_new_joint_party() {
        let joint_details = create_test_joint_details();
        let member1_id = PartyId::new_v7();
        let member2_id = PartyId::new_v7();

        let mut member1 = PartyMember::new_owner(member1_id, dec!(50));
        member1.is_primary_contact = true;
        let member2 = PartyMember::new_owner(member2_id, dec!(50));

        let party = Party::new_joint(joint_details, vec![member1, member2]);

        assert_eq!(party.party_type, PartyType::Joint);
        assert_eq!(party.composition, PartyComposition::Joint);
        assert!(party.joint_details.is_some());
        assert_eq!(party.members.len(), 2);
    }

    #[test]
    fn test_joint_display_name() {
        let joint_details = create_test_joint_details();
        let party = Party::new_joint(joint_details, vec![]);

        assert_eq!(party.display_name(), "John & Jane Doe");
    }

    #[test]
    fn test_joint_is_composite() {
        let party = Party::new_joint(create_test_joint_details(), vec![]);
        assert!(party.is_composite());
    }

    #[test]
    fn test_joint_total_ownership() {
        let member1 = PartyMember::new_owner(PartyId::new_v7(), dec!(60));
        let member2 = PartyMember::new_owner(PartyId::new_v7(), dec!(40));

        let party = Party::new_joint(
            create_test_joint_details(),
            vec![member1, member2],
        );

        assert_eq!(party.total_ownership_percentage(), dec!(100));
    }

    #[test]
    fn test_joint_primary_contact() {
        let member1_id = PartyId::new_v7();
        let member2_id = PartyId::new_v7();

        let mut member1 = PartyMember::new_owner(member1_id, dec!(50));
        member1.is_primary_contact = true;
        let member2 = PartyMember::new_owner(member2_id, dec!(50));

        let party = Party::new_joint(
            create_test_joint_details(),
            vec![member1, member2],
        );

        let primary = party.primary_contact().unwrap();
        assert_eq!(primary.member_party_id, member1_id);
    }

    #[test]
    fn test_joint_add_member() {
        let mut party = Party::new_joint(
            create_test_joint_details(),
            vec![PartyMember::new_owner(PartyId::new_v7(), dec!(100))],
        );

        let new_member = PartyMember::new_owner(PartyId::new_v7(), dec!(50));
        let result = party.add_member(new_member);

        assert!(result.is_ok());
        assert_eq!(party.members.len(), 2);
    }

    #[test]
    fn test_joint_remove_member() {
        let member1_id = PartyId::new_v7();
        let member2_id = PartyId::new_v7();

        let member1 = PartyMember::new_owner(member1_id, dec!(50));
        let member2 = PartyMember::new_owner(member2_id, dec!(50));

        let mut party = Party::new_joint(
            create_test_joint_details(),
            vec![member1, member2],
        );

        let removed = party.remove_member(member1_id);
        assert!(removed);

        let active = party.active_members();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].member_party_id, member2_id);
    }

    #[test]
    fn test_joint_types() {
        let types = vec![
            JointType::JointTenants,
            JointType::TenantsInCommon,
            JointType::CommunityProperty,
            JointType::Other("Custom".to_string()),
        ];

        for joint_type in types {
            let json = serde_json::to_string(&joint_type).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_joint_decision_makers() {
        let member1 = PartyMember::new(
            PartyId::new_v7(),
            MemberRole::PrimaryOwner,
            Some(dec!(50)),
        );
        let member2 = PartyMember::new(
            PartyId::new_v7(),
            MemberRole::CoOwner,
            Some(dec!(50)),
        );

        let party = Party::new_joint(
            create_test_joint_details(),
            vec![member1, member2],
        );

        let decision_makers = party.decision_makers();
        assert_eq!(decision_makers.len(), 2);
    }
}

// ============================================================================
// Trust Party Tests
// ============================================================================

mod trust_party_tests {
    use super::*;

    #[test]
    fn test_new_trust_party() {
        let trust_details = create_test_trust_details();
        let trustee = PartyMember::new_trustee(PartyId::new_v7());

        let party = Party::new_trust(trust_details, vec![trustee]);

        assert_eq!(party.party_type, PartyType::Trust);
        assert_eq!(party.composition, PartyComposition::Trust);
        assert!(party.trust_details.is_some());
        assert_eq!(party.members.len(), 1);
    }

    #[test]
    fn test_trust_display_name() {
        let party = Party::new_trust(create_test_trust_details(), vec![]);

        assert_eq!(party.display_name(), "Doe Family Trust");
    }

    #[test]
    fn test_trust_is_composite() {
        let party = Party::new_trust(create_test_trust_details(), vec![]);
        assert!(party.is_composite());
    }

    #[test]
    fn test_trust_with_multiple_trustees() {
        let trustee1 = PartyMember::new_trustee(PartyId::new_v7());
        let trustee2 = PartyMember::new_trustee(PartyId::new_v7());

        let party = Party::new_trust(
            create_test_trust_details(),
            vec![trustee1, trustee2],
        );

        assert_eq!(party.members.len(), 2);
    }

    #[test]
    fn test_trust_with_beneficiary() {
        let trustee = PartyMember::new_trustee(PartyId::new_v7());
        let beneficiary = PartyMember::new(
            PartyId::new_v7(),
            MemberRole::TrustBeneficiary,
            None,
        );

        let party = Party::new_trust(
            create_test_trust_details(),
            vec![trustee, beneficiary],
        );

        assert_eq!(party.members.len(), 2);
    }

    #[test]
    fn test_trust_types() {
        let types = vec![
            TrustType::RevocableLiving,
            TrustType::ILIT,
            TrustType::CharitableRemainder,
            TrustType::SpecialNeeds,
            TrustType::Testamentary,
            TrustType::Other("Custom".to_string()),
        ];

        for trust_type in types {
            let json = serde_json::to_string(&trust_type).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_trust_decision_makers() {
        let trustee = PartyMember::new_trustee(PartyId::new_v7());
        let beneficiary = PartyMember::new(
            PartyId::new_v7(),
            MemberRole::TrustBeneficiary,
            None,
        );

        let party = Party::new_trust(
            create_test_trust_details(),
            vec![trustee, beneficiary],
        );

        // Only trustee is a decision maker
        let decision_makers = party.decision_makers();
        assert_eq!(decision_makers.len(), 1);
        assert_eq!(decision_makers[0].role, MemberRole::Trustee);
    }
}

// ============================================================================
// Partnership Party Tests
// ============================================================================

mod partnership_party_tests {
    use super::*;

    #[test]
    fn test_new_partnership_party() {
        let partnership_details = create_test_partnership_details();
        let partner = PartyMember::new_partner(PartyId::new_v7(), dec!(100));

        let party = Party::new_partnership(partnership_details, vec![partner]);

        assert_eq!(party.party_type, PartyType::Partnership);
        assert_eq!(party.composition, PartyComposition::Partnership);
        assert!(party.partnership_details.is_some());
        assert_eq!(party.members.len(), 1);
    }

    #[test]
    fn test_partnership_display_name() {
        let party = Party::new_partnership(create_test_partnership_details(), vec![]);

        assert_eq!(party.display_name(), "Doe & Smith LLP");
    }

    #[test]
    fn test_partnership_is_composite() {
        let party = Party::new_partnership(create_test_partnership_details(), vec![]);
        assert!(party.is_composite());
    }

    #[test]
    fn test_partnership_with_multiple_partners() {
        let managing = PartyMember::new(
            PartyId::new_v7(),
            MemberRole::ManagingPartner,
            Some(dec!(40)),
        );
        let partner = PartyMember::new_partner(PartyId::new_v7(), dec!(35));
        let silent = PartyMember::new(
            PartyId::new_v7(),
            MemberRole::SilentPartner,
            Some(dec!(25)),
        );

        let party = Party::new_partnership(
            create_test_partnership_details(),
            vec![managing, partner, silent],
        );

        assert_eq!(party.members.len(), 3);
        assert_eq!(party.total_ownership_percentage(), dec!(100));
    }

    #[test]
    fn test_partnership_types() {
        let types = vec![
            PartnershipType::GeneralPartnership,
            PartnershipType::LimitedPartnership,
            PartnershipType::LLP,
            PartnershipType::Other("Custom".to_string()),
        ];

        for partnership_type in types {
            let json = serde_json::to_string(&partnership_type).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_partnership_decision_makers() {
        let managing = PartyMember::new(
            PartyId::new_v7(),
            MemberRole::ManagingPartner,
            Some(dec!(50)),
        );
        let silent = PartyMember::new(
            PartyId::new_v7(),
            MemberRole::SilentPartner,
            Some(dec!(50)),
        );

        let party = Party::new_partnership(
            create_test_partnership_details(),
            vec![managing, silent],
        );

        // Managing partner is decision maker, silent partner is not
        let decision_makers = party.decision_makers();
        assert_eq!(decision_makers.len(), 1);
        assert_eq!(decision_makers[0].role, MemberRole::ManagingPartner);
    }
}

// ============================================================================
// Party Member Tests
// ============================================================================

mod party_member_tests {
    use super::*;

    #[test]
    fn test_member_new() {
        let party_id = PartyId::new_v7();
        let member = PartyMember::new(
            party_id,
            MemberRole::CoOwner,
            Some(dec!(50)),
        );

        assert_eq!(member.member_party_id, party_id);
        assert_eq!(member.role, MemberRole::CoOwner);
        assert_eq!(member.ownership_percentage, Some(dec!(50)));
        assert!(!member.is_primary_contact);
        assert!(member.effective_to.is_none());
    }

    #[test]
    fn test_member_new_owner() {
        let party_id = PartyId::new_v7();
        let member = PartyMember::new_owner(party_id, dec!(75));

        assert_eq!(member.role, MemberRole::CoOwner);
        assert_eq!(member.ownership_percentage, Some(dec!(75)));
    }

    #[test]
    fn test_member_new_trustee() {
        let party_id = PartyId::new_v7();
        let member = PartyMember::new_trustee(party_id);

        assert_eq!(member.role, MemberRole::Trustee);
        assert!(member.ownership_percentage.is_none());
    }

    #[test]
    fn test_member_new_partner() {
        let party_id = PartyId::new_v7();
        let member = PartyMember::new_partner(party_id, dec!(33.33));

        assert_eq!(member.role, MemberRole::Partner);
        assert_eq!(member.ownership_percentage, Some(dec!(33.33)));
    }

    #[test]
    fn test_member_is_active() {
        let member = PartyMember::new_owner(PartyId::new_v7(), dec!(50));
        assert!(member.is_active());
    }

    #[test]
    fn test_member_terminate() {
        let mut member = PartyMember::new_owner(PartyId::new_v7(), dec!(50));
        member.terminate();

        assert!(member.effective_to.is_some());
        assert!(!member.is_active());
    }

    #[test]
    fn test_member_role_has_decision_authority() {
        assert!(MemberRole::PrimaryOwner.has_decision_authority());
        assert!(MemberRole::CoOwner.has_decision_authority());
        assert!(MemberRole::Trustee.has_decision_authority());
        assert!(MemberRole::ManagingPartner.has_decision_authority());
        assert!(MemberRole::Partner.has_decision_authority());
        assert!(MemberRole::AuthorizedSignatory.has_decision_authority());
        assert!(MemberRole::Director.has_decision_authority());

        assert!(!MemberRole::TrustBeneficiary.has_decision_authority());
        assert!(!MemberRole::Settlor.has_decision_authority());
        assert!(!MemberRole::SilentPartner.has_decision_authority());
    }

    #[test]
    fn test_member_role_is_owner() {
        assert!(MemberRole::PrimaryOwner.is_owner());
        assert!(MemberRole::CoOwner.is_owner());
        assert!(MemberRole::ManagingPartner.is_owner());
        assert!(MemberRole::Partner.is_owner());
        assert!(MemberRole::SilentPartner.is_owner());

        assert!(!MemberRole::Trustee.is_owner());
        assert!(!MemberRole::TrustBeneficiary.is_owner());
        assert!(!MemberRole::Settlor.is_owner());
        assert!(!MemberRole::AuthorizedSignatory.is_owner());
        assert!(!MemberRole::Director.is_owner());
    }
}

// ============================================================================
// Party Type Conversion Tests
// ============================================================================

mod party_type_tests {
    use super::*;

    #[test]
    fn test_party_composition_to_party_type() {
        assert_eq!(
            PartyType::from(PartyComposition::Individual),
            PartyType::Individual
        );
        assert_eq!(
            PartyType::from(PartyComposition::Corporate),
            PartyType::Corporate
        );
        assert_eq!(
            PartyType::from(PartyComposition::Joint),
            PartyType::Joint
        );
        assert_eq!(
            PartyType::from(PartyComposition::Trust),
            PartyType::Trust
        );
        assert_eq!(
            PartyType::from(PartyComposition::Partnership),
            PartyType::Partnership
        );
    }

    #[test]
    fn test_all_party_types_serializable() {
        let types = vec![
            PartyType::Individual,
            PartyType::Corporate,
            PartyType::Agent,
            PartyType::Broker,
            PartyType::Joint,
            PartyType::Trust,
            PartyType::Partnership,
        ];

        for party_type in types {
            let json = serde_json::to_string(&party_type).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_all_compositions_serializable() {
        let compositions = vec![
            PartyComposition::Individual,
            PartyComposition::Corporate,
            PartyComposition::Joint,
            PartyComposition::Trust,
            PartyComposition::Partnership,
        ];

        for composition in compositions {
            let json = serde_json::to_string(&composition).unwrap();
            assert!(!json.is_empty());
        }
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
    fn test_party_primary_address() {
        let mut party = Party::new_individual(create_test_individual());

        let mut addr1 = Address::new(
            AddressType::Residential,
            "123 Main St",
            "New York",
            "10001",
            "USA",
        );
        addr1.is_primary = true;

        let addr2 = Address::new(
            AddressType::Residential,
            "456 Oak Ave",
            "Brooklyn",
            "11201",
            "USA",
        );

        party.add_address(addr1);
        party.add_address(addr2);

        let primary = party.primary_address(AddressType::Residential);
        assert!(primary.is_some());
        assert_eq!(primary.unwrap().line1, "123 Main St");
    }

    #[test]
    fn test_address_format() {
        let mut address = Address::new(
            AddressType::Residential,
            "123 Main St",
            "New York",
            "10001",
            "USA",
        );
        address.state = Some("NY".to_string());

        let formatted = address.format();

        assert!(formatted.contains("123 Main St"));
        assert!(formatted.contains("New York, NY 10001"));
        assert!(formatted.contains("USA"));
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
    fn test_kyc_document_is_expired() {
        let party_id = PartyId::new_v7();
        let mut doc = KycDocument::new(party_id, DocumentType::Passport);

        // Not expired (future date)
        doc.expiry_date = Some(Utc::now().date_naive() + Days::new(365));
        assert!(!doc.is_expired());

        // Expired (past date)
        doc.expiry_date = Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap());
        assert!(doc.is_expired());

        // No expiry
        doc.expiry_date = None;
        assert!(!doc.is_expired());
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
    use core_kernel::AgentId;

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
    fn test_agent_is_licensed() {
        let mut agent = Agent::new(PartyId::new_v7(), "AGT001");

        // No license required
        assert!(agent.is_licensed());

        // Valid license
        agent.license_number = Some("LIC-12345".to_string());
        agent.license_expiry = Some(Utc::now().date_naive() + Days::new(365));
        assert!(agent.is_licensed());

        // Expired license
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
    fn test_agent_with_manager() {
        let mut agent = Agent::new(PartyId::new_v7(), "AGT001");
        let manager_id = AgentId::new_v7();
        agent.manager_id = Some(manager_id);

        assert_eq!(agent.manager_id, Some(manager_id));
    }
}

// ============================================================================
// Validation Tests
// ============================================================================

mod validation_tests {
    use super::*;

    #[test]
    fn test_valid_individual_party() {
        let party = Party::new_individual(create_test_individual());
        let result = PartyValidator::validate(&party);
        assert!(result.is_valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_individual_empty_name() {
        let mut individual = create_test_individual();
        individual.first_name = "".to_string();
        let party = Party::new_individual(individual);
        let result = PartyValidator::validate(&party);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_valid_joint_party() {
        let joint_details = create_test_joint_details();
        let mut member1 = PartyMember::new_owner(PartyId::new_v7(), dec!(50));
        member1.is_primary_contact = true;
        let member2 = PartyMember::new_owner(PartyId::new_v7(), dec!(50));

        let party = Party::new_joint(joint_details, vec![member1, member2]);
        let result = PartyValidator::validate(&party);
        assert!(result.is_valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_joint_single_member() {
        let joint_details = create_test_joint_details();
        let member = PartyMember::new_owner(PartyId::new_v7(), dec!(100));

        let party = Party::new_joint(joint_details, vec![member]);
        let result = PartyValidator::validate(&party);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("at least 2")));
    }

    #[test]
    fn test_invalid_joint_ownership_not_100() {
        let joint_details = create_test_joint_details();
        let member1 = PartyMember::new_owner(PartyId::new_v7(), dec!(40));
        let member2 = PartyMember::new_owner(PartyId::new_v7(), dec!(40));

        let party = Party::new_joint(joint_details, vec![member1, member2]);
        let result = PartyValidator::validate(&party);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("100%")));
    }

    #[test]
    fn test_valid_trust_party() {
        let trust_details = create_test_trust_details();
        let trustee = PartyMember::new_trustee(PartyId::new_v7());

        let party = Party::new_trust(trust_details, vec![trustee]);
        let result = PartyValidator::validate(&party);
        assert!(result.is_valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_trust_no_trustee() {
        let trust_details = create_test_trust_details();
        let party = Party::new_trust(trust_details, vec![]);
        let result = PartyValidator::validate(&party);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("trustee")));
    }

    #[test]
    fn test_valid_partnership_party() {
        let partnership_details = create_test_partnership_details();
        let partner = PartyMember::new(
            PartyId::new_v7(),
            MemberRole::ManagingPartner,
            Some(dec!(100)),
        );

        let party = Party::new_partnership(partnership_details, vec![partner]);
        let result = PartyValidator::validate(&party);
        assert!(result.is_valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_partnership_no_partner() {
        let partnership_details = create_test_partnership_details();
        let party = Party::new_partnership(partnership_details, vec![]);
        let result = PartyValidator::validate(&party);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("partner")));
    }

    #[test]
    fn test_validate_as_policyholder_kyc_failed() {
        let mut party = Party::new_individual(create_test_individual());
        party.kyc_status = KycStatus::Failed;

        let result = PartyValidator::validate_as_policyholder(&party);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("KYC")));
    }

    #[test]
    fn test_validate_as_policyholder_inactive() {
        let mut party = Party::new_individual(create_test_individual());
        party.is_active = false;

        let result = PartyValidator::validate_as_policyholder(&party);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("Inactive")));
    }

    #[test]
    fn test_validate_member_invalid_role_for_joint() {
        let member = PartyMember::new_trustee(PartyId::new_v7());
        let result = PartyValidator::validate_member(&member, &PartyComposition::Joint);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_member_valid_role_for_trust() {
        let member = PartyMember::new_trustee(PartyId::new_v7());
        let result = PartyValidator::validate_member(&member, &PartyComposition::Trust);
        assert!(result.is_valid);
    }
}

// ============================================================================
// Error Tests
// ============================================================================

mod error_tests {
    use domain_party::error::PartyError;

    #[test]
    fn test_party_error_not_found() {
        let error = PartyError::not_found("123");
        assert!(error.to_string().contains("not found"));
    }

    #[test]
    fn test_party_error_invalid() {
        let error = PartyError::invalid("bad data");
        assert!(error.to_string().contains("Invalid"));
    }

    #[test]
    fn test_party_error_validation_failed() {
        let error = PartyError::validation_failed(vec![
            "Error 1".to_string(),
            "Error 2".to_string(),
        ]);
        let msg = error.to_string();
        assert!(msg.contains("Error 1"));
        assert!(msg.contains("Error 2"));
    }

    #[test]
    fn test_party_error_invalid_composition() {
        let error = PartyError::invalid_composition("wrong type");
        assert!(error.to_string().contains("composition"));
    }

    #[test]
    fn test_party_error_invalid_ownership() {
        let error = PartyError::invalid_ownership("must be 100%");
        assert!(error.to_string().contains("ownership"));
    }
}
