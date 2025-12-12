//! Party validation rules
//!
//! This module provides comprehensive validation for party entities,
//! ensuring data integrity and business rule compliance.
//!
//! # Validation Rules
//!
//! ## Individual Parties
//! - Must have first name and last name
//! - Date of birth must be in the past
//! - Age must be reasonable (0-150 years)
//!
//! ## Corporate Parties
//! - Must have company name
//! - Incorporation date must be in the past (if provided)
//!
//! ## Joint Parties
//! - Must have at least 2 members
//! - All members must have ownership roles
//! - Total ownership percentage must equal 100%
//! - Must have a display name
//!
//! ## Trust Parties
//! - Must have at least 1 trustee
//! - Must have trust name
//!
//! ## Partnership Parties
//! - Must have at least 1 partner
//! - Total ownership percentage should equal 100%
//! - Must have partnership name

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use chrono::Utc;

use crate::party::{Party, PartyComposition, MemberRole};
use crate::error::PartyError;

/// Result of party validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the party is valid
    pub is_valid: bool,
    /// List of validation errors
    pub errors: Vec<String>,
    /// List of validation warnings (non-fatal issues)
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Creates a successful validation result
    pub fn ok() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Creates a failed validation result with errors
    pub fn fail(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Adds an error to the result
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
        self.is_valid = false;
    }

    /// Adds a warning to the result
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Merges another validation result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        if !other.is_valid {
            self.is_valid = false;
        }
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::ok()
    }
}

/// Validator for party entities
///
/// Provides comprehensive validation for all party types,
/// ensuring business rules and data integrity constraints are met.
///
/// # Examples
///
/// ```rust
/// use domain_party::validation::PartyValidator;
/// use domain_party::party::Party;
///
/// let party = Party::new_individual(/* ... */);
/// let result = PartyValidator::validate(&party);
///
/// if !result.is_valid {
///     for error in result.errors {
///         println!("Validation error: {}", error);
///     }
/// }
/// ```
pub struct PartyValidator;

impl PartyValidator {
    /// Validates a party according to its composition type
    ///
    /// # Arguments
    ///
    /// * `party` - The party to validate
    ///
    /// # Returns
    ///
    /// A `ValidationResult` containing any errors or warnings
    pub fn validate(party: &Party) -> ValidationResult {
        let mut result = ValidationResult::ok();

        // Common validations
        Self::validate_common(party, &mut result);

        // Type-specific validations
        match party.composition {
            PartyComposition::Individual => Self::validate_individual(party, &mut result),
            PartyComposition::Corporate => Self::validate_corporate(party, &mut result),
            PartyComposition::Joint => Self::validate_joint(party, &mut result),
            PartyComposition::Trust => Self::validate_trust(party, &mut result),
            PartyComposition::Partnership => Self::validate_partnership(party, &mut result),
        }

        result
    }

    /// Validates common party attributes
    fn validate_common(party: &Party, result: &mut ValidationResult) {
        // Email format validation (basic)
        if let Some(ref email) = party.email {
            if !email.contains('@') || !email.contains('.') {
                result.add_error(format!("Invalid email format: {}", email));
            }
        }

        // Phone format validation (basic - just check it's not empty if provided)
        if let Some(ref phone) = party.phone {
            if phone.trim().is_empty() {
                result.add_error("Phone number cannot be empty");
            }
        }

        // Check addresses for primary designation
        let primary_addresses: Vec<_> = party.addresses.iter().filter(|a| a.is_primary).collect();
        if primary_addresses.len() > 1 {
            result.add_warning("Multiple primary addresses found");
        }
    }

    /// Validates individual party attributes
    fn validate_individual(party: &Party, result: &mut ValidationResult) {
        match &party.individual {
            Some(individual) => {
                // Name validation
                if individual.first_name.trim().is_empty() {
                    result.add_error("Individual first name is required");
                }
                if individual.last_name.trim().is_empty() {
                    result.add_error("Individual last name is required");
                }

                // Date of birth validation
                let today = Utc::now().date_naive();
                if individual.date_of_birth > today {
                    result.add_error("Date of birth cannot be in the future");
                }

                // Age validation (reasonable range)
                let age = individual.age();
                if age > 150 {
                    result.add_error(format!("Invalid age: {} years", age));
                }

                // Nationality format (should be 2-letter ISO code)
                if let Some(ref nationality) = individual.nationality {
                    if nationality.len() != 2 {
                        result.add_warning("Nationality should be a 2-letter ISO country code");
                    }
                }
            }
            None => {
                result.add_error("Individual party must have individual details");
            }
        }

        // Individual should not have members
        if !party.members.is_empty() {
            result.add_error("Individual party should not have members");
        }
    }

    /// Validates corporate party attributes
    fn validate_corporate(party: &Party, result: &mut ValidationResult) {
        match &party.corporate {
            Some(corporate) => {
                // Company name validation
                if corporate.company_name.trim().is_empty() {
                    result.add_error("Corporate party must have a company name");
                }

                // Incorporation date validation
                if let Some(inc_date) = corporate.incorporation_date {
                    let today = Utc::now().date_naive();
                    if inc_date > today {
                        result.add_error("Incorporation date cannot be in the future");
                    }
                }

                // Country format validation
                if let Some(ref country) = corporate.incorporation_country {
                    if country.len() != 2 {
                        result.add_warning("Incorporation country should be a 2-letter ISO code");
                    }
                }
            }
            None => {
                result.add_error("Corporate party must have corporate details");
            }
        }

        // Corporate may have members (authorized signatories, directors)
        // but they should have appropriate roles
        for member in &party.members {
            if !matches!(
                member.role,
                MemberRole::AuthorizedSignatory | MemberRole::Director
            ) {
                result.add_warning(format!(
                    "Unusual role {:?} for corporate party member",
                    member.role
                ));
            }
        }
    }

    /// Validates joint party attributes
    fn validate_joint(party: &Party, result: &mut ValidationResult) {
        match &party.joint_details {
            Some(joint_details) => {
                // Display name validation
                if joint_details.display_name.trim().is_empty() {
                    result.add_error("Joint party must have a display name");
                }
            }
            None => {
                result.add_error("Joint party must have joint details");
            }
        }

        // Member count validation
        let active_members: Vec<_> = party.members.iter().filter(|m| m.is_active()).collect();
        if active_members.len() < 2 {
            result.add_error("Joint party must have at least 2 active members");
        }

        // Ownership role validation
        for member in &active_members {
            if !member.role.is_owner() {
                result.add_error(format!(
                    "Joint party members must have ownership roles, found {:?}",
                    member.role
                ));
            }

            // Ownership percentage required for joint members
            if member.ownership_percentage.is_none() {
                result.add_error("Joint party members must have ownership percentage");
            }
        }

        // Total ownership validation
        let total_ownership = party.total_ownership_percentage();
        if total_ownership != dec!(100) {
            result.add_error(format!(
                "Joint party ownership must total 100%, found {}%",
                total_ownership
            ));
        }

        // Validate ownership percentages are positive
        for member in &active_members {
            if let Some(pct) = member.ownership_percentage {
                if pct <= dec!(0) {
                    result.add_error("Ownership percentage must be positive");
                }
                if pct > dec!(100) {
                    result.add_error("Ownership percentage cannot exceed 100%");
                }
            }
        }

        // Check for primary contact
        if party.primary_contact().is_none() && !active_members.is_empty() {
            result.add_warning("Joint party should have a primary contact designated");
        }
    }

    /// Validates trust party attributes
    fn validate_trust(party: &Party, result: &mut ValidationResult) {
        match &party.trust_details {
            Some(trust_details) => {
                // Trust name validation
                if trust_details.trust_name.trim().is_empty() {
                    result.add_error("Trust party must have a trust name");
                }

                // Established date validation
                if let Some(est_date) = trust_details.established_date {
                    let today = Utc::now().date_naive();
                    if est_date > today {
                        result.add_error("Trust established date cannot be in the future");
                    }
                }
            }
            None => {
                result.add_error("Trust party must have trust details");
            }
        }

        // Trustee validation
        let trustees: Vec<_> = party
            .members
            .iter()
            .filter(|m| m.is_active() && matches!(m.role, MemberRole::Trustee))
            .collect();

        if trustees.is_empty() {
            result.add_error("Trust party must have at least 1 active trustee");
        }

        // Validate all members have appropriate trust roles
        for member in &party.members {
            if member.is_active() {
                match member.role {
                    MemberRole::Trustee
                    | MemberRole::TrustBeneficiary
                    | MemberRole::Settlor => {}
                    _ => {
                        result.add_warning(format!(
                            "Unusual role {:?} for trust party member",
                            member.role
                        ));
                    }
                }
            }
        }
    }

    /// Validates partnership party attributes
    fn validate_partnership(party: &Party, result: &mut ValidationResult) {
        match &party.partnership_details {
            Some(partnership_details) => {
                // Partnership name validation
                if partnership_details.partnership_name.trim().is_empty() {
                    result.add_error("Partnership party must have a partnership name");
                }

                // Formation date validation
                if let Some(form_date) = partnership_details.formation_date {
                    let today = Utc::now().date_naive();
                    if form_date > today {
                        result.add_error("Partnership formation date cannot be in the future");
                    }
                }
            }
            None => {
                result.add_error("Partnership party must have partnership details");
            }
        }

        // Partner validation
        let partners: Vec<_> = party
            .members
            .iter()
            .filter(|m| {
                m.is_active()
                    && matches!(
                        m.role,
                        MemberRole::ManagingPartner | MemberRole::Partner | MemberRole::SilentPartner
                    )
            })
            .collect();

        if partners.is_empty() {
            result.add_error("Partnership party must have at least 1 active partner");
        }

        // Total ownership validation
        let total_ownership = party.total_ownership_percentage();
        if total_ownership > dec!(0) && total_ownership != dec!(100) {
            result.add_warning(format!(
                "Partnership ownership should total 100%, found {}%",
                total_ownership
            ));
        }

        // Validate ownership percentages for partners
        for member in &partners {
            if let Some(pct) = member.ownership_percentage {
                if pct < dec!(0) {
                    result.add_error("Ownership percentage cannot be negative");
                }
                if pct > dec!(100) {
                    result.add_error("Ownership percentage cannot exceed 100%");
                }
            }
        }

        // Managing partner check
        let managing_partners: Vec<_> = partners
            .iter()
            .filter(|m| matches!(m.role, MemberRole::ManagingPartner))
            .collect();

        if managing_partners.is_empty() {
            result.add_warning("Partnership should have at least one managing partner");
        }
    }

    /// Validates that a party can be used as a policyholder
    ///
    /// Additional checks for parties that will own insurance policies.
    ///
    /// # Arguments
    ///
    /// * `party` - The party to validate
    ///
    /// # Returns
    ///
    /// A `ValidationResult` for policyholder eligibility
    pub fn validate_as_policyholder(party: &Party) -> ValidationResult {
        let mut result = Self::validate(party);

        // KYC must be verified or in progress
        use crate::kyc::KycStatus;
        match party.kyc_status {
            KycStatus::Verified | KycStatus::InProgress => {}
            KycStatus::Pending => {
                result.add_warning("KYC verification is pending");
            }
            KycStatus::Failed => {
                result.add_error("Party failed KYC verification");
            }
            KycStatus::Expired => {
                result.add_error("Party KYC verification has expired");
            }
        }

        // Must be active
        if !party.is_active {
            result.add_error("Inactive party cannot be a policyholder");
        }

        // Composite parties must have decision makers
        if party.is_composite() && party.decision_makers().is_empty() {
            result.add_error("Composite party must have at least one decision maker");
        }

        result
    }

    /// Validates a party member
    ///
    /// # Arguments
    ///
    /// * `member` - The member to validate
    /// * `party_composition` - The composition type of the parent party
    ///
    /// # Returns
    ///
    /// A `ValidationResult` for the member
    pub fn validate_member(
        member: &crate::party::PartyMember,
        party_composition: &PartyComposition,
    ) -> ValidationResult {
        let mut result = ValidationResult::ok();

        // Ownership percentage validation
        if let Some(pct) = member.ownership_percentage {
            if pct < dec!(0) {
                result.add_error("Ownership percentage cannot be negative");
            }
            if pct > dec!(100) {
                result.add_error("Ownership percentage cannot exceed 100%");
            }
        }

        // Role-composition compatibility
        match party_composition {
            PartyComposition::Joint => {
                if !matches!(member.role, MemberRole::PrimaryOwner | MemberRole::CoOwner) {
                    result.add_error(format!(
                        "Invalid role {:?} for joint party member",
                        member.role
                    ));
                }
                if member.ownership_percentage.is_none() {
                    result.add_error("Joint party members must have ownership percentage");
                }
            }
            PartyComposition::Trust => {
                if !matches!(
                    member.role,
                    MemberRole::Trustee | MemberRole::TrustBeneficiary | MemberRole::Settlor
                ) {
                    result.add_warning(format!(
                        "Unusual role {:?} for trust party member",
                        member.role
                    ));
                }
            }
            PartyComposition::Partnership => {
                if !matches!(
                    member.role,
                    MemberRole::ManagingPartner | MemberRole::Partner | MemberRole::SilentPartner
                ) {
                    result.add_warning(format!(
                        "Unusual role {:?} for partnership party member",
                        member.role
                    ));
                }
            }
            PartyComposition::Corporate => {
                if !matches!(
                    member.role,
                    MemberRole::AuthorizedSignatory | MemberRole::Director
                ) {
                    result.add_warning(format!(
                        "Unusual role {:?} for corporate party member",
                        member.role
                    ));
                }
            }
            PartyComposition::Individual => {
                result.add_error("Individual parties should not have members");
            }
        }

        // Effective dates validation
        if let Some(end) = member.effective_to {
            if end <= member.effective_from {
                result.add_error("Member effective_to must be after effective_from");
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::party::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use core_kernel::PartyId;

    fn create_valid_individual() -> Individual {
        Individual {
            first_name: "John".to_string(),
            middle_name: None,
            last_name: "Doe".to_string(),
            date_of_birth: NaiveDate::from_ymd_opt(1985, 6, 15).unwrap(),
            gender: Some(Gender::Male),
            nationality: Some("US".to_string()),
            tax_id: None,
            occupation: None,
        }
    }

    fn create_valid_corporate() -> Corporate {
        Corporate {
            company_name: "Acme Corp".to_string(),
            registration_number: Some("REG123".to_string()),
            tax_id: None,
            industry: None,
            incorporation_date: Some(NaiveDate::from_ymd_opt(2010, 1, 1).unwrap()),
            incorporation_country: Some("US".to_string()),
            corporate_type: None,
        }
    }

    #[test]
    fn test_valid_individual() {
        let party = Party::new_individual(create_valid_individual());
        let result = PartyValidator::validate(&party);
        assert!(result.is_valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_individual_missing_name() {
        let mut individual = create_valid_individual();
        individual.first_name = "".to_string();
        let party = Party::new_individual(individual);
        let result = PartyValidator::validate(&party);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("first name")));
    }

    #[test]
    fn test_valid_corporate() {
        let party = Party::new_corporate(create_valid_corporate());
        let result = PartyValidator::validate(&party);
        assert!(result.is_valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_corporate_missing_name() {
        let mut corporate = create_valid_corporate();
        corporate.company_name = "".to_string();
        let party = Party::new_corporate(corporate);
        let result = PartyValidator::validate(&party);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("company name")));
    }

    #[test]
    fn test_valid_joint_party() {
        let joint_details = JointDetails {
            display_name: "John & Jane Smith".to_string(),
            joint_type: JointType::JointTenants,
            notes: None,
        };

        let mut member1 = PartyMember::new_owner(PartyId::new_v7(), dec!(50));
        member1.is_primary_contact = true;
        let member2 = PartyMember::new_owner(PartyId::new_v7(), dec!(50));

        let party = Party::new_joint(joint_details, vec![member1, member2]);
        let result = PartyValidator::validate(&party);
        assert!(result.is_valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_joint_party_single_member() {
        let joint_details = JointDetails {
            display_name: "John Smith".to_string(),
            joint_type: JointType::JointTenants,
            notes: None,
        };

        let member = PartyMember::new_owner(PartyId::new_v7(), dec!(100));
        let party = Party::new_joint(joint_details, vec![member]);
        let result = PartyValidator::validate(&party);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("at least 2")));
    }

    #[test]
    fn test_invalid_joint_party_ownership_not_100() {
        let joint_details = JointDetails {
            display_name: "John & Jane".to_string(),
            joint_type: JointType::JointTenants,
            notes: None,
        };

        let member1 = PartyMember::new_owner(PartyId::new_v7(), dec!(40));
        let member2 = PartyMember::new_owner(PartyId::new_v7(), dec!(40));

        let party = Party::new_joint(joint_details, vec![member1, member2]);
        let result = PartyValidator::validate(&party);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("100%")));
    }

    #[test]
    fn test_valid_trust_party() {
        let trust_details = TrustDetails {
            trust_name: "Smith Family Trust".to_string(),
            trust_id: None,
            established_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            trust_type: TrustType::RevocableLiving,
            is_revocable: true,
            governing_jurisdiction: Some("CA".to_string()),
        };

        let trustee = PartyMember::new_trustee(PartyId::new_v7());
        let party = Party::new_trust(trust_details, vec![trustee]);
        let result = PartyValidator::validate(&party);
        assert!(result.is_valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_trust_no_trustee() {
        let trust_details = TrustDetails {
            trust_name: "Smith Family Trust".to_string(),
            trust_id: None,
            established_date: None,
            trust_type: TrustType::RevocableLiving,
            is_revocable: true,
            governing_jurisdiction: None,
        };

        let party = Party::new_trust(trust_details, vec![]);
        let result = PartyValidator::validate(&party);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("trustee")));
    }

    #[test]
    fn test_valid_partnership_party() {
        let partnership_details = PartnershipDetails {
            partnership_name: "Smith & Jones LLP".to_string(),
            registration_number: None,
            tax_id: None,
            partnership_type: PartnershipType::LLP,
            formation_date: None,
            formation_jurisdiction: None,
        };

        let partner1 = PartyMember::new(
            PartyId::new_v7(),
            MemberRole::ManagingPartner,
            Some(dec!(60)),
        );
        let partner2 = PartyMember::new_partner(PartyId::new_v7(), dec!(40));

        let party = Party::new_partnership(partnership_details, vec![partner1, partner2]);
        let result = PartyValidator::validate(&party);
        assert!(result.is_valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_validate_as_policyholder_kyc_failed() {
        let mut party = Party::new_individual(create_valid_individual());
        party.kyc_status = crate::kyc::KycStatus::Failed;

        let result = PartyValidator::validate_as_policyholder(&party);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("KYC")));
    }

    #[test]
    fn test_validate_as_policyholder_inactive() {
        let mut party = Party::new_individual(create_valid_individual());
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
}
