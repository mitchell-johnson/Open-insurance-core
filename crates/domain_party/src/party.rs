//! Party entity and composition model
//!
//! This module defines the Party aggregate which represents any entity that can
//! participate in insurance contracts. Parties can be individuals, corporations,
//! or composite structures like joint owners, trusts, or partnerships.
//!
//! # Party Composition
//!
//! A Party can be composed of multiple members, enabling complex ownership structures:
//!
//! - **Individual**: A single natural person
//! - **Corporate**: A single legal entity (company, LLC, etc.)
//! - **Joint**: Multiple individuals sharing ownership (e.g., married couple)
//! - **Trust**: A trust entity with trustees who manage it
//! - **Partnership**: A business partnership with individual partners
//!
//! # Examples
//!
//! ```rust
//! use domain_party::party::{Party, PartyComposition, Individual, JointDetails};
//! use rust_decimal_macros::dec;
//!
//! // Create a joint ownership party for a married couple
//! let husband = Individual { first_name: "John".into(), last_name: "Smith".into(), .. };
//! let wife = Individual { first_name: "Jane".into(), last_name: "Smith".into(), .. };
//!
//! let joint_party = Party::new_joint(
//!     JointDetails { name: "John & Jane Smith".into(), .. },
//!     vec![
//!         PartyMember::new_owner(husband_party_id, dec!(50)),
//!         PartyMember::new_owner(wife_party_id, dec!(50)),
//!     ],
//! );
//! ```

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

use core_kernel::PartyId;
use crate::address::Address;
use crate::kyc::KycStatus;

/// The composition type of a party, determining its structure and membership rules.
///
/// This enum defines the fundamental nature of a party entity:
/// - Simple parties (Individual, Corporate) represent single entities
/// - Composite parties (Joint, Trust, Partnership) contain multiple member parties
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartyComposition {
    /// A single natural person
    Individual,
    /// A single legal entity (company, LLC, corporation, etc.)
    Corporate,
    /// Multiple individuals sharing joint ownership
    ///
    /// Used for married couples, business partners, or any group of individuals
    /// who wish to hold a policy jointly. All members must be individuals.
    Joint,
    /// A trust entity managed by trustees
    ///
    /// The trust itself is the legal owner, but trustees (individuals or corporates)
    /// manage and make decisions on behalf of the trust.
    Trust,
    /// A business partnership with individual partners
    ///
    /// Combines a corporate structure with individual partners who have
    /// ownership stakes and decision-making authority.
    Partnership,
}

/// Legacy party type for backwards compatibility
///
/// Maps to the new PartyComposition model but maintains API compatibility
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartyType {
    Individual,
    Corporate,
    Agent,
    Broker,
    /// Joint ownership (multiple individuals)
    Joint,
    /// Trust with trustees
    Trust,
    /// Partnership with partners
    Partnership,
}

impl From<PartyComposition> for PartyType {
    fn from(composition: PartyComposition) -> Self {
        match composition {
            PartyComposition::Individual => PartyType::Individual,
            PartyComposition::Corporate => PartyType::Corporate,
            PartyComposition::Joint => PartyType::Joint,
            PartyComposition::Trust => PartyType::Trust,
            PartyComposition::Partnership => PartyType::Partnership,
        }
    }
}

/// The role a member plays within a composite party.
///
/// Different party compositions have different valid member roles:
/// - Joint: PrimaryOwner, CoOwner
/// - Trust: Trustee, Beneficiary, Settlor
/// - Partnership: ManagingPartner, Partner, SilentPartner
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberRole {
    // Joint ownership roles
    /// Primary owner in a joint ownership arrangement
    PrimaryOwner,
    /// Co-owner with equal or specified ownership percentage
    CoOwner,

    // Trust roles
    /// Trustee who manages the trust
    Trustee,
    /// Beneficiary of the trust (for informational purposes)
    TrustBeneficiary,
    /// Settlor who established the trust
    Settlor,

    // Partnership roles
    /// Managing partner with decision-making authority
    ManagingPartner,
    /// General partner with ownership and liability
    Partner,
    /// Limited/silent partner with ownership but limited involvement
    SilentPartner,

    // Corporate roles
    /// Authorized signatory for corporate entities
    AuthorizedSignatory,
    /// Director of a corporate entity
    Director,
}

impl MemberRole {
    /// Returns whether this role grants decision-making authority
    ///
    /// Decision-makers can authorize policy changes, claims, and other
    /// significant actions on behalf of the party.
    ///
    /// # Returns
    ///
    /// `true` if this role can make decisions for the party
    pub fn has_decision_authority(&self) -> bool {
        matches!(
            self,
            MemberRole::PrimaryOwner
                | MemberRole::CoOwner
                | MemberRole::Trustee
                | MemberRole::ManagingPartner
                | MemberRole::Partner
                | MemberRole::AuthorizedSignatory
                | MemberRole::Director
        )
    }

    /// Returns whether this role represents an owner
    ///
    /// Owners have financial stake in the party and typically receive
    /// proceeds from insurance benefits.
    ///
    /// # Returns
    ///
    /// `true` if this role represents ownership
    pub fn is_owner(&self) -> bool {
        matches!(
            self,
            MemberRole::PrimaryOwner
                | MemberRole::CoOwner
                | MemberRole::ManagingPartner
                | MemberRole::Partner
                | MemberRole::SilentPartner
        )
    }
}

/// A member of a composite party.
///
/// Members link individual or corporate parties to a composite party structure.
/// Each member has a role defining their relationship and an optional ownership
/// percentage for joint ownership scenarios.
///
/// # Examples
///
/// ```rust
/// use domain_party::party::{PartyMember, MemberRole};
/// use rust_decimal_macros::dec;
/// use core_kernel::PartyId;
///
/// // Create a 50% co-owner
/// let member = PartyMember {
///     member_party_id: PartyId::new_v7(),
///     role: MemberRole::CoOwner,
///     ownership_percentage: Some(dec!(50)),
///     is_primary_contact: false,
///     effective_from: chrono::Utc::now(),
///     effective_to: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMember {
    /// The party ID of the member (must reference an Individual or Corporate party)
    pub member_party_id: PartyId,
    /// The role this member plays in the composite party
    pub role: MemberRole,
    /// Ownership percentage (0-100), required for ownership roles
    pub ownership_percentage: Option<Decimal>,
    /// Whether this member is the primary contact for communications
    pub is_primary_contact: bool,
    /// When this membership became effective
    pub effective_from: DateTime<Utc>,
    /// When this membership ended (None if currently active)
    pub effective_to: Option<DateTime<Utc>>,
}

impl PartyMember {
    /// Creates a new party member with the specified role and ownership
    ///
    /// # Arguments
    ///
    /// * `member_party_id` - The party ID of the member
    /// * `role` - The role this member plays
    /// * `ownership_percentage` - Optional ownership percentage (0-100)
    ///
    /// # Returns
    ///
    /// A new `PartyMember` instance with current timestamp as effective_from
    pub fn new(
        member_party_id: PartyId,
        role: MemberRole,
        ownership_percentage: Option<Decimal>,
    ) -> Self {
        Self {
            member_party_id,
            role,
            ownership_percentage,
            is_primary_contact: false,
            effective_from: Utc::now(),
            effective_to: None,
        }
    }

    /// Creates a new owner member with the specified ownership percentage
    ///
    /// Convenience method for creating ownership members in joint parties.
    ///
    /// # Arguments
    ///
    /// * `member_party_id` - The party ID of the owner
    /// * `ownership_percentage` - The ownership percentage (0-100)
    ///
    /// # Returns
    ///
    /// A new `PartyMember` with CoOwner role
    pub fn new_owner(member_party_id: PartyId, ownership_percentage: Decimal) -> Self {
        Self::new(member_party_id, MemberRole::CoOwner, Some(ownership_percentage))
    }

    /// Creates a new trustee member
    ///
    /// # Arguments
    ///
    /// * `member_party_id` - The party ID of the trustee
    ///
    /// # Returns
    ///
    /// A new `PartyMember` with Trustee role
    pub fn new_trustee(member_party_id: PartyId) -> Self {
        Self::new(member_party_id, MemberRole::Trustee, None)
    }

    /// Creates a new partner member
    ///
    /// # Arguments
    ///
    /// * `member_party_id` - The party ID of the partner
    /// * `ownership_percentage` - The partnership percentage (0-100)
    ///
    /// # Returns
    ///
    /// A new `PartyMember` with Partner role
    pub fn new_partner(member_party_id: PartyId, ownership_percentage: Decimal) -> Self {
        Self::new(member_party_id, MemberRole::Partner, Some(ownership_percentage))
    }

    /// Checks if this membership is currently active
    ///
    /// # Returns
    ///
    /// `true` if effective_to is None or in the future
    pub fn is_active(&self) -> bool {
        self.effective_to.map_or(true, |end| end > Utc::now())
    }

    /// Ends this membership as of the current time
    pub fn terminate(&mut self) {
        self.effective_to = Some(Utc::now());
    }
}

/// Individual person details
///
/// Contains personal information for natural persons participating
/// in insurance contracts.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Individual {
    /// Legal first name
    pub first_name: String,
    /// Middle name(s), if any
    pub middle_name: Option<String>,
    /// Legal last name / surname
    pub last_name: String,
    /// Date of birth for age calculations and mortality tables
    pub date_of_birth: NaiveDate,
    /// Gender for actuarial purposes
    pub gender: Option<Gender>,
    /// Country of nationality (ISO 3166-1 alpha-2)
    pub nationality: Option<String>,
    /// Tax identification number
    pub tax_id: Option<String>,
    /// Current occupation
    pub occupation: Option<String>,
}

impl Individual {
    /// Returns the full name of the individual
    ///
    /// # Returns
    ///
    /// Full name in "First Middle Last" format
    pub fn full_name(&self) -> String {
        match &self.middle_name {
            Some(middle) => format!("{} {} {}", self.first_name, middle, self.last_name),
            None => format!("{} {}", self.first_name, self.last_name),
        }
    }

    /// Calculates the current age of the individual
    ///
    /// # Returns
    ///
    /// Age in years as of today
    pub fn age(&self) -> u32 {
        let today = Utc::now().date_naive();
        let mut age = today.year() - self.date_of_birth.year();

        // Adjust if birthday hasn't occurred yet this year
        if today.ordinal() < self.date_of_birth.ordinal() {
            age -= 1;
        }

        age as u32
    }
}

/// Gender enumeration for actuarial purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Other,
}

/// Corporate entity details
///
/// Contains information for legal entities such as companies,
/// LLCs, corporations, and other non-natural persons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Corporate {
    /// Legal company name
    pub company_name: String,
    /// Company registration/incorporation number
    pub registration_number: Option<String>,
    /// Tax identification number (EIN, TIN, etc.)
    pub tax_id: Option<String>,
    /// Industry classification
    pub industry: Option<String>,
    /// Date of incorporation
    pub incorporation_date: Option<NaiveDate>,
    /// Country of incorporation (ISO 3166-1 alpha-2)
    pub incorporation_country: Option<String>,
    /// Type of corporate structure
    pub corporate_type: Option<CorporateType>,
}

/// Types of corporate structures
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorporateType {
    /// Limited Liability Company
    LLC,
    /// Corporation (C-Corp, S-Corp)
    Corporation,
    /// Sole Proprietorship
    SoleProprietorship,
    /// Non-profit organization
    NonProfit,
    /// Government entity
    Government,
    /// Other corporate structure
    Other(String),
}

/// Details specific to joint ownership arrangements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointDetails {
    /// Display name for the joint ownership (e.g., "John & Jane Smith")
    pub display_name: String,
    /// Type of joint ownership arrangement
    pub joint_type: JointType,
    /// Notes about the arrangement
    pub notes: Option<String>,
}

/// Types of joint ownership arrangements
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JointType {
    /// Joint Tenants with Right of Survivorship (JTWROS)
    /// On death, ownership passes to surviving owner(s)
    JointTenants,
    /// Tenants in Common (TIC)
    /// Ownership shares pass to each owner's estate
    TenantsInCommon,
    /// Community Property (married couples in community property states)
    CommunityProperty,
    /// Other arrangement
    Other(String),
}

/// Details specific to trust entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustDetails {
    /// Legal name of the trust
    pub trust_name: String,
    /// Trust identification number
    pub trust_id: Option<String>,
    /// Date the trust was established
    pub established_date: Option<NaiveDate>,
    /// Type of trust
    pub trust_type: TrustType,
    /// Whether the trust is revocable
    pub is_revocable: bool,
    /// State/jurisdiction governing the trust
    pub governing_jurisdiction: Option<String>,
}

/// Types of trusts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustType {
    /// Revocable Living Trust
    RevocableLiving,
    /// Irrevocable Life Insurance Trust (ILIT)
    ILIT,
    /// Charitable Remainder Trust
    CharitableRemainder,
    /// Special Needs Trust
    SpecialNeeds,
    /// Testamentary Trust
    Testamentary,
    /// Other trust type
    Other(String),
}

/// Details specific to partnership entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartnershipDetails {
    /// Partnership name
    pub partnership_name: String,
    /// Partnership registration number
    pub registration_number: Option<String>,
    /// Tax identification number
    pub tax_id: Option<String>,
    /// Type of partnership
    pub partnership_type: PartnershipType,
    /// Date partnership was formed
    pub formation_date: Option<NaiveDate>,
    /// State/jurisdiction of formation
    pub formation_jurisdiction: Option<String>,
}

/// Types of partnerships
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartnershipType {
    /// General Partnership
    GeneralPartnership,
    /// Limited Partnership (LP)
    LimitedPartnership,
    /// Limited Liability Partnership (LLP)
    LLP,
    /// Other partnership type
    Other(String),
}

/// A party (person, organization, or composite entity)
///
/// Party is the core aggregate representing any entity that can participate
/// in insurance contracts. It supports both simple entities (individuals,
/// companies) and composite structures (joint owners, trusts, partnerships).
///
/// # Ownership Model
///
/// Policies reference a single `policyholder_id`. By making Party support
/// composite structures, we maintain simple policy ownership while enabling:
///
/// - Joint individual owners (married couples, business partners)
/// - Corporate ownership with authorized signatories
/// - Trust ownership with trustees
/// - Partnership ownership with partners
///
/// # Examples
///
/// ```rust
/// use domain_party::party::{Party, Individual, Gender};
/// use chrono::NaiveDate;
///
/// let individual = Individual {
///     first_name: "John".to_string(),
///     middle_name: None,
///     last_name: "Doe".to_string(),
///     date_of_birth: NaiveDate::from_ymd_opt(1985, 6, 15).unwrap(),
///     gender: Some(Gender::Male),
///     nationality: Some("US".to_string()),
///     tax_id: None,
///     occupation: Some("Engineer".to_string()),
/// };
///
/// let party = Party::new_individual(individual);
/// assert_eq!(party.display_name(), "John Doe");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    /// Unique party identifier
    pub id: PartyId,
    /// Legacy party type (for backwards compatibility)
    pub party_type: PartyType,
    /// The composition type of this party
    pub composition: PartyComposition,
    /// Individual details (if composition is Individual)
    pub individual: Option<Individual>,
    /// Corporate details (if composition is Corporate)
    pub corporate: Option<Corporate>,
    /// Joint ownership details (if composition is Joint)
    pub joint_details: Option<JointDetails>,
    /// Trust details (if composition is Trust)
    pub trust_details: Option<TrustDetails>,
    /// Partnership details (if composition is Partnership)
    pub partnership_details: Option<PartnershipDetails>,
    /// Members of this party (for composite types: Joint, Trust, Partnership)
    #[serde(default)]
    pub members: Vec<PartyMember>,
    /// Postal addresses
    #[serde(default)]
    pub addresses: Vec<Address>,
    /// Primary email address
    pub email: Option<String>,
    /// Primary phone number
    pub phone: Option<String>,
    /// Preferred language (ISO 639-1)
    pub preferred_language: Option<String>,
    /// KYC verification status
    pub kyc_status: KycStatus,
    /// Whether this party is active
    pub is_active: bool,
    /// When this party was created
    pub created_at: DateTime<Utc>,
    /// When this party was last updated
    pub updated_at: DateTime<Utc>,
}

impl Party {
    /// Creates a new individual party
    ///
    /// # Arguments
    ///
    /// * `individual` - The individual's personal details
    ///
    /// # Returns
    ///
    /// A new `Party` with Individual composition
    ///
    /// # Examples
    ///
    /// ```rust
    /// let party = Party::new_individual(Individual { .. });
    /// assert_eq!(party.composition, PartyComposition::Individual);
    /// ```
    pub fn new_individual(individual: Individual) -> Self {
        let now = Utc::now();
        Self {
            id: PartyId::new_v7(),
            party_type: PartyType::Individual,
            composition: PartyComposition::Individual,
            individual: Some(individual),
            corporate: None,
            joint_details: None,
            trust_details: None,
            partnership_details: None,
            members: Vec::new(),
            addresses: Vec::new(),
            email: None,
            phone: None,
            preferred_language: None,
            kyc_status: KycStatus::Pending,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new corporate party
    ///
    /// # Arguments
    ///
    /// * `corporate` - The corporate entity details
    ///
    /// # Returns
    ///
    /// A new `Party` with Corporate composition
    pub fn new_corporate(corporate: Corporate) -> Self {
        let now = Utc::now();
        Self {
            id: PartyId::new_v7(),
            party_type: PartyType::Corporate,
            composition: PartyComposition::Corporate,
            individual: None,
            corporate: Some(corporate),
            joint_details: None,
            trust_details: None,
            partnership_details: None,
            members: Vec::new(),
            addresses: Vec::new(),
            email: None,
            phone: None,
            preferred_language: None,
            kyc_status: KycStatus::Pending,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new joint ownership party
    ///
    /// Joint parties represent multiple individuals who jointly own something,
    /// such as a married couple or business partners.
    ///
    /// # Arguments
    ///
    /// * `joint_details` - Details about the joint arrangement
    /// * `members` - The individual party members (must be at least 2)
    ///
    /// # Returns
    ///
    /// A new `Party` with Joint composition
    ///
    /// # Examples
    ///
    /// ```rust
    /// let joint_party = Party::new_joint(
    ///     JointDetails {
    ///         display_name: "John & Jane Smith".to_string(),
    ///         joint_type: JointType::JointTenants,
    ///         notes: None,
    ///     },
    ///     vec![
    ///         PartyMember::new_owner(husband_id, dec!(50)),
    ///         PartyMember::new_owner(wife_id, dec!(50)),
    ///     ],
    /// );
    /// ```
    pub fn new_joint(joint_details: JointDetails, members: Vec<PartyMember>) -> Self {
        let now = Utc::now();
        Self {
            id: PartyId::new_v7(),
            party_type: PartyType::Joint,
            composition: PartyComposition::Joint,
            individual: None,
            corporate: None,
            joint_details: Some(joint_details),
            trust_details: None,
            partnership_details: None,
            members,
            addresses: Vec::new(),
            email: None,
            phone: None,
            preferred_language: None,
            kyc_status: KycStatus::Pending,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new trust party
    ///
    /// Trust parties represent legal trusts with trustees who manage them.
    ///
    /// # Arguments
    ///
    /// * `trust_details` - Details about the trust
    /// * `trustees` - The trustee party members (must be at least 1)
    ///
    /// # Returns
    ///
    /// A new `Party` with Trust composition
    pub fn new_trust(trust_details: TrustDetails, trustees: Vec<PartyMember>) -> Self {
        let now = Utc::now();
        Self {
            id: PartyId::new_v7(),
            party_type: PartyType::Trust,
            composition: PartyComposition::Trust,
            individual: None,
            corporate: None,
            joint_details: None,
            trust_details: Some(trust_details),
            partnership_details: None,
            members: trustees,
            addresses: Vec::new(),
            email: None,
            phone: None,
            preferred_language: None,
            kyc_status: KycStatus::Pending,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new partnership party
    ///
    /// Partnership parties represent business partnerships with individual partners.
    ///
    /// # Arguments
    ///
    /// * `partnership_details` - Details about the partnership
    /// * `partners` - The partner party members (must be at least 1)
    ///
    /// # Returns
    ///
    /// A new `Party` with Partnership composition
    pub fn new_partnership(partnership_details: PartnershipDetails, partners: Vec<PartyMember>) -> Self {
        let now = Utc::now();
        Self {
            id: PartyId::new_v7(),
            party_type: PartyType::Partnership,
            composition: PartyComposition::Partnership,
            individual: None,
            corporate: None,
            joint_details: None,
            trust_details: None,
            partnership_details: Some(partnership_details),
            members: partners,
            addresses: Vec::new(),
            email: None,
            phone: None,
            preferred_language: None,
            kyc_status: KycStatus::Pending,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Returns the display name for this party
    ///
    /// The display name varies based on the party composition:
    /// - Individual: "First Last"
    /// - Corporate: Company name
    /// - Joint: Joint display name (e.g., "John & Jane Smith")
    /// - Trust: Trust name
    /// - Partnership: Partnership name
    ///
    /// # Returns
    ///
    /// A human-readable display name for the party
    pub fn display_name(&self) -> String {
        match &self.composition {
            PartyComposition::Individual => {
                self.individual
                    .as_ref()
                    .map(|i| format!("{} {}", i.first_name, i.last_name))
                    .unwrap_or_else(|| "Unknown".to_string())
            }
            PartyComposition::Corporate => {
                self.corporate
                    .as_ref()
                    .map(|c| c.company_name.clone())
                    .unwrap_or_else(|| "Unknown".to_string())
            }
            PartyComposition::Joint => {
                self.joint_details
                    .as_ref()
                    .map(|j| j.display_name.clone())
                    .unwrap_or_else(|| "Joint Owners".to_string())
            }
            PartyComposition::Trust => {
                self.trust_details
                    .as_ref()
                    .map(|t| t.trust_name.clone())
                    .unwrap_or_else(|| "Trust".to_string())
            }
            PartyComposition::Partnership => {
                self.partnership_details
                    .as_ref()
                    .map(|p| p.partnership_name.clone())
                    .unwrap_or_else(|| "Partnership".to_string())
            }
        }
    }

    /// Checks if this party is a composite (has members)
    ///
    /// # Returns
    ///
    /// `true` if this party can have members (Joint, Trust, or Partnership)
    pub fn is_composite(&self) -> bool {
        matches!(
            self.composition,
            PartyComposition::Joint | PartyComposition::Trust | PartyComposition::Partnership
        )
    }

    /// Returns all active members of this party
    ///
    /// For simple parties (Individual, Corporate), returns an empty vector.
    /// For composite parties, returns only members with no end date or future end date.
    ///
    /// # Returns
    ///
    /// Vector of active party members
    pub fn active_members(&self) -> Vec<&PartyMember> {
        self.members.iter().filter(|m| m.is_active()).collect()
    }

    /// Returns members with decision-making authority
    ///
    /// These are members who can authorize policy changes, claims,
    /// and other significant actions.
    ///
    /// # Returns
    ///
    /// Vector of members with decision authority
    pub fn decision_makers(&self) -> Vec<&PartyMember> {
        self.members
            .iter()
            .filter(|m| m.is_active() && m.role.has_decision_authority())
            .collect()
    }

    /// Returns the primary contact member, if any
    ///
    /// # Returns
    ///
    /// The member designated as primary contact, or None
    pub fn primary_contact(&self) -> Option<&PartyMember> {
        self.members.iter().find(|m| m.is_primary_contact && m.is_active())
    }

    /// Returns the total ownership percentage of all members
    ///
    /// For valid joint ownership, this should equal 100.
    ///
    /// # Returns
    ///
    /// Sum of all member ownership percentages
    pub fn total_ownership_percentage(&self) -> Decimal {
        self.members
            .iter()
            .filter(|m| m.is_active())
            .filter_map(|m| m.ownership_percentage)
            .sum()
    }

    /// Adds a member to this composite party
    ///
    /// # Arguments
    ///
    /// * `member` - The member to add
    ///
    /// # Errors
    ///
    /// Returns error if this is not a composite party or member is invalid
    pub fn add_member(&mut self, member: PartyMember) -> Result<(), String> {
        if !self.is_composite() {
            return Err("Cannot add members to non-composite party".to_string());
        }
        self.members.push(member);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Removes a member from this composite party
    ///
    /// Does not actually remove the member, but sets their effective_to date.
    ///
    /// # Arguments
    ///
    /// * `member_party_id` - The party ID of the member to remove
    ///
    /// # Returns
    ///
    /// `true` if a member was found and terminated
    pub fn remove_member(&mut self, member_party_id: PartyId) -> bool {
        let mut found = false;
        for member in &mut self.members {
            if member.member_party_id == member_party_id && member.is_active() {
                member.terminate();
                found = true;
            }
        }
        if found {
            self.updated_at = Utc::now();
        }
        found
    }

    /// Adds an address to this party
    ///
    /// # Arguments
    ///
    /// * `address` - The address to add
    pub fn add_address(&mut self, address: Address) {
        self.addresses.push(address);
        self.updated_at = Utc::now();
    }

    /// Returns the primary address of the specified type, if any
    ///
    /// # Arguments
    ///
    /// * `address_type` - The type of address to find
    ///
    /// # Returns
    ///
    /// The primary address of that type, or None
    pub fn primary_address(&self, address_type: crate::address::AddressType) -> Option<&Address> {
        self.addresses
            .iter()
            .filter(|a| a.address_type == address_type)
            .find(|a| a.is_primary)
    }
}

use chrono::Datelike;
