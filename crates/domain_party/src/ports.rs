//! Party Domain Ports
//!
//! This module defines the port interfaces for the party domain, enabling
//! swappable implementations (internal database, external CRM, mock, etc.).
//!
//! # Architecture
//!
//! The `PartyPort` trait defines all operations that the party domain needs
//! from its data source. Multiple adapters can implement this trait:
//!
//! - **Internal Adapter**: Uses PostgreSQL database (infra_db)
//! - **External API Adapter**: Calls an external CRM or customer system
//! - **Mock Adapter**: For testing without external dependencies
//!
//! # Usage
//!
//! ```rust,ignore
//! use domain_party::ports::PartyPort;
//! use std::sync::Arc;
//!
//! // Application services receive the port trait
//! pub struct PartyService {
//!     party_port: Arc<dyn PartyPort>,
//! }
//!
//! impl PartyService {
//!     pub async fn get_party(&self, id: PartyId) -> Result<Party, PortError> {
//!         self.party_port.get_party(id, None).await
//!     }
//! }
//! ```
//!
//! # Configuration
//!
//! Adapters are configured at application startup. The choice of adapter can be
//! driven by environment configuration:
//!
//! ```rust,ignore
//! match config.party_source {
//!     PartySource::Internal => Arc::new(PostgresPartyAdapter::new(pool)),
//!     PartySource::ExternalCrm => Arc::new(CrmPartyAdapter::new(crm_config)),
//! }
//! ```

use async_trait::async_trait;
use rust_decimal::Decimal;

use core_kernel::{PartyId, PortError, DomainPort, OperationMetadata, HealthCheckable, HealthCheckResult};

use crate::party::{
    Party, PartyComposition, PartyMember, MemberRole,
    Individual, Corporate,
    JointDetails, JointType,
    TrustDetails, TrustType,
    PartnershipDetails, PartnershipType,
};
use crate::kyc::KycStatus;

/// Query parameters for finding parties
#[derive(Debug, Clone, Default)]
pub struct PartyQuery {
    /// Filter by email address
    pub email: Option<String>,
    /// Filter by phone number
    pub phone: Option<String>,
    /// Filter by party composition type
    pub composition: Option<PartyComposition>,
    /// Filter by KYC status
    pub kyc_status: Option<KycStatus>,
    /// Filter by active status
    pub is_active: Option<bool>,
    /// Limit results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

impl PartyQuery {
    /// Creates a query to find by email
    pub fn by_email(email: impl Into<String>) -> Self {
        Self {
            email: Some(email.into()),
            ..Default::default()
        }
    }

    /// Creates a query to find by composition type
    pub fn by_composition(composition: PartyComposition) -> Self {
        Self {
            composition: Some(composition),
            ..Default::default()
        }
    }

    /// Adds pagination to the query
    pub fn paginate(mut self, limit: u32, offset: u32) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }
}

/// Request for creating a new party
#[derive(Debug, Clone)]
pub struct CreatePartyRequest {
    /// The party composition type
    pub composition: PartyComposition,
    /// Individual details (required if composition is Individual)
    pub individual: Option<Individual>,
    /// Corporate details (required if composition is Corporate)
    pub corporate: Option<Corporate>,
    /// Joint details (required if composition is Joint)
    pub joint_details: Option<JointDetails>,
    /// Trust details (required if composition is Trust)
    pub trust_details: Option<TrustDetails>,
    /// Partnership details (required if composition is Partnership)
    pub partnership_details: Option<PartnershipDetails>,
    /// Initial members (for composite parties)
    pub members: Vec<CreateMemberRequest>,
    /// Email address
    pub email: Option<String>,
    /// Phone number
    pub phone: Option<String>,
}

/// Request for creating a party member
#[derive(Debug, Clone)]
pub struct CreateMemberRequest {
    /// The party ID of the member
    pub member_party_id: PartyId,
    /// Role in the composite party
    pub role: MemberRole,
    /// Ownership percentage (0-100)
    pub ownership_percentage: Option<Decimal>,
    /// Whether this is the primary contact
    pub is_primary_contact: bool,
}

/// Request for updating a party
#[derive(Debug, Clone, Default)]
pub struct UpdatePartyRequest {
    /// New email address
    pub email: Option<String>,
    /// New phone number
    pub phone: Option<String>,
    /// New KYC status
    pub kyc_status: Option<KycStatus>,
    /// Whether the party is active
    pub is_active: Option<bool>,
}

/// The main port trait for party domain operations
///
/// This trait defines all operations that the party domain requires from its
/// underlying data source. Implementations can be internal (database) or
/// external (API to external system).
///
/// All methods are async and return `Result<T, PortError>` for consistent
/// error handling across different adapter implementations.
#[async_trait]
pub trait PartyPort: DomainPort + HealthCheckable {
    // ========================================================================
    // Basic CRUD Operations
    // ========================================================================

    /// Retrieves a party by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The party identifier
    /// * `metadata` - Optional operation metadata for tracing/auditing
    ///
    /// # Returns
    ///
    /// The party if found, or `PortError::NotFound`
    async fn get_party(
        &self,
        id: PartyId,
        metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError>;

    /// Retrieves multiple parties by their IDs
    ///
    /// # Arguments
    ///
    /// * `ids` - The party identifiers
    /// * `metadata` - Optional operation metadata
    ///
    /// # Returns
    ///
    /// A vector of parties (may be fewer than requested if some not found)
    async fn get_parties(
        &self,
        ids: Vec<PartyId>,
        metadata: Option<OperationMetadata>,
    ) -> Result<Vec<Party>, PortError>;

    /// Finds parties matching the query criteria
    ///
    /// # Arguments
    ///
    /// * `query` - Query parameters for filtering
    /// * `metadata` - Optional operation metadata
    ///
    /// # Returns
    ///
    /// A vector of matching parties
    async fn find_parties(
        &self,
        query: PartyQuery,
        metadata: Option<OperationMetadata>,
    ) -> Result<Vec<Party>, PortError>;

    /// Creates a new party
    ///
    /// # Arguments
    ///
    /// * `request` - The party creation request
    /// * `metadata` - Optional operation metadata
    ///
    /// # Returns
    ///
    /// The created party with generated ID
    async fn create_party(
        &self,
        request: CreatePartyRequest,
        metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError>;

    /// Updates an existing party
    ///
    /// # Arguments
    ///
    /// * `id` - The party identifier
    /// * `request` - The update request
    /// * `metadata` - Optional operation metadata
    ///
    /// # Returns
    ///
    /// The updated party
    async fn update_party(
        &self,
        id: PartyId,
        request: UpdatePartyRequest,
        metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError>;

    /// Deactivates a party (soft delete)
    ///
    /// # Arguments
    ///
    /// * `id` - The party identifier
    /// * `metadata` - Optional operation metadata
    async fn deactivate_party(
        &self,
        id: PartyId,
        metadata: Option<OperationMetadata>,
    ) -> Result<(), PortError>;

    // ========================================================================
    // Member Operations (for composite parties)
    // ========================================================================

    /// Gets all active members of a composite party
    ///
    /// # Arguments
    ///
    /// * `party_id` - The composite party ID
    /// * `metadata` - Optional operation metadata
    ///
    /// # Returns
    ///
    /// Vector of active members
    async fn get_members(
        &self,
        party_id: PartyId,
        metadata: Option<OperationMetadata>,
    ) -> Result<Vec<PartyMember>, PortError>;

    /// Adds a member to a composite party
    ///
    /// # Arguments
    ///
    /// * `party_id` - The composite party ID
    /// * `request` - The member creation request
    /// * `metadata` - Optional operation metadata
    ///
    /// # Returns
    ///
    /// The created member
    async fn add_member(
        &self,
        party_id: PartyId,
        request: CreateMemberRequest,
        metadata: Option<OperationMetadata>,
    ) -> Result<PartyMember, PortError>;

    /// Removes a member from a composite party
    ///
    /// # Arguments
    ///
    /// * `party_id` - The composite party ID
    /// * `member_party_id` - The member's party ID
    /// * `metadata` - Optional operation metadata
    async fn remove_member(
        &self,
        party_id: PartyId,
        member_party_id: PartyId,
        metadata: Option<OperationMetadata>,
    ) -> Result<(), PortError>;

    /// Updates a member's ownership percentage
    ///
    /// # Arguments
    ///
    /// * `party_id` - The composite party ID
    /// * `member_party_id` - The member's party ID
    /// * `new_percentage` - The new ownership percentage
    /// * `metadata` - Optional operation metadata
    async fn update_member_ownership(
        &self,
        party_id: PartyId,
        member_party_id: PartyId,
        new_percentage: Decimal,
        metadata: Option<OperationMetadata>,
    ) -> Result<PartyMember, PortError>;

    /// Sets the primary contact for a composite party
    ///
    /// # Arguments
    ///
    /// * `party_id` - The composite party ID
    /// * `member_party_id` - The member's party ID to set as primary
    /// * `metadata` - Optional operation metadata
    async fn set_primary_contact(
        &self,
        party_id: PartyId,
        member_party_id: PartyId,
        metadata: Option<OperationMetadata>,
    ) -> Result<(), PortError>;

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// Finds all composite parties that contain a specific member
    ///
    /// # Arguments
    ///
    /// * `member_party_id` - The member's party ID
    /// * `metadata` - Optional operation metadata
    ///
    /// # Returns
    ///
    /// Vector of composite parties containing this member
    async fn find_by_member(
        &self,
        member_party_id: PartyId,
        metadata: Option<OperationMetadata>,
    ) -> Result<Vec<Party>, PortError>;

    /// Checks if a party exists
    ///
    /// # Arguments
    ///
    /// * `id` - The party identifier
    /// * `metadata` - Optional operation metadata
    ///
    /// # Returns
    ///
    /// True if the party exists
    async fn exists(
        &self,
        id: PartyId,
        metadata: Option<OperationMetadata>,
    ) -> Result<bool, PortError>;

    // ========================================================================
    // KYC Operations
    // ========================================================================

    /// Updates the KYC status of a party
    ///
    /// # Arguments
    ///
    /// * `id` - The party identifier
    /// * `status` - The new KYC status
    /// * `metadata` - Optional operation metadata
    async fn update_kyc_status(
        &self,
        id: PartyId,
        status: KycStatus,
        metadata: Option<OperationMetadata>,
    ) -> Result<(), PortError>;
}

/// Extension trait for PartyPort with convenience methods
#[async_trait]
pub trait PartyPortExt: PartyPort {
    /// Gets a party or returns NotFound error
    async fn get_party_required(
        &self,
        id: PartyId,
        metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError> {
        self.get_party(id, metadata).await
    }

    /// Finds a single party by email
    async fn find_by_email(
        &self,
        email: &str,
        metadata: Option<OperationMetadata>,
    ) -> Result<Option<Party>, PortError> {
        let parties = self.find_parties(PartyQuery::by_email(email), metadata).await?;
        Ok(parties.into_iter().next())
    }

    /// Creates an individual party
    async fn create_individual(
        &self,
        individual: Individual,
        email: Option<String>,
        phone: Option<String>,
        metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError> {
        self.create_party(
            CreatePartyRequest {
                composition: PartyComposition::Individual,
                individual: Some(individual),
                corporate: None,
                joint_details: None,
                trust_details: None,
                partnership_details: None,
                members: vec![],
                email,
                phone,
            },
            metadata,
        )
        .await
    }

    /// Creates a corporate party
    async fn create_corporate(
        &self,
        corporate: Corporate,
        email: Option<String>,
        phone: Option<String>,
        metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError> {
        self.create_party(
            CreatePartyRequest {
                composition: PartyComposition::Corporate,
                individual: None,
                corporate: Some(corporate),
                joint_details: None,
                trust_details: None,
                partnership_details: None,
                members: vec![],
                email,
                phone,
            },
            metadata,
        )
        .await
    }

    /// Creates a joint ownership party
    async fn create_joint(
        &self,
        joint_details: JointDetails,
        members: Vec<CreateMemberRequest>,
        email: Option<String>,
        metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError> {
        self.create_party(
            CreatePartyRequest {
                composition: PartyComposition::Joint,
                individual: None,
                corporate: None,
                joint_details: Some(joint_details),
                trust_details: None,
                partnership_details: None,
                members,
                email,
                phone: None,
            },
            metadata,
        )
        .await
    }
}

// Blanket implementation for all PartyPort implementors
impl<T: PartyPort> PartyPortExt for T {}

/// Mock implementation of PartyPort for testing
///
/// This adapter stores parties in memory and is useful for unit testing
/// without database or external API dependencies.
#[cfg(any(test, feature = "mock"))]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use chrono::Utc;

    /// In-memory mock implementation of PartyPort
    #[derive(Debug, Default)]
    pub struct MockPartyPort {
        parties: Arc<RwLock<HashMap<PartyId, Party>>>,
        members: Arc<RwLock<HashMap<PartyId, Vec<PartyMember>>>>,
    }

    impl MockPartyPort {
        /// Creates a new mock port
        pub fn new() -> Self {
            Self::default()
        }

        /// Pre-populates with parties for testing
        pub async fn with_parties(parties: Vec<Party>) -> Self {
            let port = Self::new();
            for party in parties {
                port.parties.write().await.insert(party.id, party);
            }
            port
        }
    }

    impl DomainPort for MockPartyPort {}

    #[async_trait]
    impl HealthCheckable for MockPartyPort {
        async fn health_check(&self) -> HealthCheckResult {
            HealthCheckResult {
                adapter_id: "mock-party-port".to_string(),
                status: core_kernel::AdapterHealth::Healthy,
                latency_ms: 0,
                message: Some("Mock adapter always healthy".to_string()),
                checked_at: Utc::now(),
            }
        }
    }

    #[async_trait]
    impl PartyPort for MockPartyPort {
        async fn get_party(
            &self,
            id: PartyId,
            _metadata: Option<OperationMetadata>,
        ) -> Result<Party, PortError> {
            self.parties
                .read()
                .await
                .get(&id)
                .cloned()
                .ok_or_else(|| PortError::not_found("Party", id))
        }

        async fn get_parties(
            &self,
            ids: Vec<PartyId>,
            _metadata: Option<OperationMetadata>,
        ) -> Result<Vec<Party>, PortError> {
            let parties = self.parties.read().await;
            Ok(ids.into_iter().filter_map(|id| parties.get(&id).cloned()).collect())
        }

        async fn find_parties(
            &self,
            query: PartyQuery,
            _metadata: Option<OperationMetadata>,
        ) -> Result<Vec<Party>, PortError> {
            let parties = self.parties.read().await;
            let mut results: Vec<_> = parties
                .values()
                .filter(|p| {
                    if let Some(ref email) = query.email {
                        if p.email.as_ref() != Some(email) {
                            return false;
                        }
                    }
                    if let Some(composition) = query.composition {
                        if p.composition != composition {
                            return false;
                        }
                    }
                    if let Some(kyc_status) = query.kyc_status {
                        if p.kyc_status != kyc_status {
                            return false;
                        }
                    }
                    if let Some(is_active) = query.is_active {
                        if p.is_active != is_active {
                            return false;
                        }
                    }
                    true
                })
                .cloned()
                .collect();

            // Apply pagination
            if let Some(offset) = query.offset {
                results = results.into_iter().skip(offset as usize).collect();
            }
            if let Some(limit) = query.limit {
                results = results.into_iter().take(limit as usize).collect();
            }

            Ok(results)
        }

        async fn create_party(
            &self,
            request: CreatePartyRequest,
            _metadata: Option<OperationMetadata>,
        ) -> Result<Party, PortError> {
            let party = match request.composition {
                PartyComposition::Individual => {
                    let individual = request.individual
                        .ok_or_else(|| PortError::validation("Individual details required"))?;
                    let mut party = Party::new_individual(individual);
                    party.email = request.email;
                    party.phone = request.phone;
                    party
                }
                PartyComposition::Corporate => {
                    let corporate = request.corporate
                        .ok_or_else(|| PortError::validation("Corporate details required"))?;
                    let mut party = Party::new_corporate(corporate);
                    party.email = request.email;
                    party.phone = request.phone;
                    party
                }
                PartyComposition::Joint => {
                    let joint_details = request.joint_details
                        .ok_or_else(|| PortError::validation("Joint details required"))?;
                    let members = request.members.into_iter().map(|m| {
                        PartyMember::new(m.member_party_id, m.role, m.ownership_percentage)
                    }).collect();
                    let mut party = Party::new_joint(joint_details, members);
                    party.email = request.email;
                    party
                }
                PartyComposition::Trust => {
                    let trust_details = request.trust_details
                        .ok_or_else(|| PortError::validation("Trust details required"))?;
                    let members = request.members.into_iter().map(|m| {
                        PartyMember::new(m.member_party_id, m.role, m.ownership_percentage)
                    }).collect();
                    let mut party = Party::new_trust(trust_details, members);
                    party.email = request.email;
                    party
                }
                PartyComposition::Partnership => {
                    let partnership_details = request.partnership_details
                        .ok_or_else(|| PortError::validation("Partnership details required"))?;
                    let members = request.members.into_iter().map(|m| {
                        PartyMember::new(m.member_party_id, m.role, m.ownership_percentage)
                    }).collect();
                    let mut party = Party::new_partnership(partnership_details, members);
                    party.email = request.email;
                    party
                }
            };

            self.parties.write().await.insert(party.id, party.clone());
            Ok(party)
        }

        async fn update_party(
            &self,
            id: PartyId,
            request: UpdatePartyRequest,
            _metadata: Option<OperationMetadata>,
        ) -> Result<Party, PortError> {
            let mut parties = self.parties.write().await;
            let party = parties.get_mut(&id)
                .ok_or_else(|| PortError::not_found("Party", id))?;

            if let Some(email) = request.email {
                party.email = Some(email);
            }
            if let Some(phone) = request.phone {
                party.phone = Some(phone);
            }
            if let Some(kyc_status) = request.kyc_status {
                party.kyc_status = kyc_status;
            }
            if let Some(is_active) = request.is_active {
                party.is_active = is_active;
            }
            party.updated_at = Utc::now();

            Ok(party.clone())
        }

        async fn deactivate_party(
            &self,
            id: PartyId,
            _metadata: Option<OperationMetadata>,
        ) -> Result<(), PortError> {
            let mut parties = self.parties.write().await;
            let party = parties.get_mut(&id)
                .ok_or_else(|| PortError::not_found("Party", id))?;
            party.is_active = false;
            party.updated_at = Utc::now();
            Ok(())
        }

        async fn get_members(
            &self,
            party_id: PartyId,
            _metadata: Option<OperationMetadata>,
        ) -> Result<Vec<PartyMember>, PortError> {
            let parties = self.parties.read().await;
            let party = parties.get(&party_id)
                .ok_or_else(|| PortError::not_found("Party", party_id))?;
            Ok(party.active_members().into_iter().cloned().collect())
        }

        async fn add_member(
            &self,
            party_id: PartyId,
            request: CreateMemberRequest,
            _metadata: Option<OperationMetadata>,
        ) -> Result<PartyMember, PortError> {
            let mut parties = self.parties.write().await;
            let party = parties.get_mut(&party_id)
                .ok_or_else(|| PortError::not_found("Party", party_id))?;

            let mut member = PartyMember::new(
                request.member_party_id,
                request.role,
                request.ownership_percentage,
            );
            member.is_primary_contact = request.is_primary_contact;

            party.add_member(member.clone())
                .map_err(|e| PortError::validation(e))?;

            Ok(member)
        }

        async fn remove_member(
            &self,
            party_id: PartyId,
            member_party_id: PartyId,
            _metadata: Option<OperationMetadata>,
        ) -> Result<(), PortError> {
            let mut parties = self.parties.write().await;
            let party = parties.get_mut(&party_id)
                .ok_or_else(|| PortError::not_found("Party", party_id))?;

            if !party.remove_member(member_party_id) {
                return Err(PortError::not_found("Member", member_party_id));
            }
            Ok(())
        }

        async fn update_member_ownership(
            &self,
            party_id: PartyId,
            member_party_id: PartyId,
            new_percentage: Decimal,
            _metadata: Option<OperationMetadata>,
        ) -> Result<PartyMember, PortError> {
            let mut parties = self.parties.write().await;
            let party = parties.get_mut(&party_id)
                .ok_or_else(|| PortError::not_found("Party", party_id))?;

            for member in &mut party.members {
                if member.member_party_id == member_party_id && member.is_active() {
                    member.ownership_percentage = Some(new_percentage);
                    return Ok(member.clone());
                }
            }
            Err(PortError::not_found("Member", member_party_id))
        }

        async fn set_primary_contact(
            &self,
            party_id: PartyId,
            member_party_id: PartyId,
            _metadata: Option<OperationMetadata>,
        ) -> Result<(), PortError> {
            let mut parties = self.parties.write().await;
            let party = parties.get_mut(&party_id)
                .ok_or_else(|| PortError::not_found("Party", party_id))?;

            let mut found = false;
            for member in &mut party.members {
                if member.member_party_id == member_party_id && member.is_active() {
                    member.is_primary_contact = true;
                    found = true;
                } else {
                    member.is_primary_contact = false;
                }
            }

            if !found {
                return Err(PortError::not_found("Member", member_party_id));
            }
            Ok(())
        }

        async fn find_by_member(
            &self,
            member_party_id: PartyId,
            _metadata: Option<OperationMetadata>,
        ) -> Result<Vec<Party>, PortError> {
            let parties = self.parties.read().await;
            Ok(parties
                .values()
                .filter(|p| {
                    p.members.iter().any(|m| m.member_party_id == member_party_id && m.is_active())
                })
                .cloned()
                .collect())
        }

        async fn exists(
            &self,
            id: PartyId,
            _metadata: Option<OperationMetadata>,
        ) -> Result<bool, PortError> {
            Ok(self.parties.read().await.contains_key(&id))
        }

        async fn update_kyc_status(
            &self,
            id: PartyId,
            status: KycStatus,
            _metadata: Option<OperationMetadata>,
        ) -> Result<(), PortError> {
            let mut parties = self.parties.write().await;
            let party = parties.get_mut(&id)
                .ok_or_else(|| PortError::not_found("Party", id))?;
            party.kyc_status = status;
            party.updated_at = Utc::now();
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::mock::MockPartyPort;
    use chrono::NaiveDate;

    fn create_test_individual() -> Individual {
        Individual {
            first_name: "John".to_string(),
            middle_name: None,
            last_name: "Doe".to_string(),
            date_of_birth: NaiveDate::from_ymd_opt(1985, 6, 15).unwrap(),
            gender: None,
            nationality: None,
            tax_id: None,
            occupation: None,
        }
    }

    #[tokio::test]
    async fn test_mock_port_create_and_get() {
        let port = MockPartyPort::new();

        let party = port.create_individual(
            create_test_individual(),
            Some("john@example.com".to_string()),
            None,
            None,
        ).await.unwrap();

        let retrieved = port.get_party(party.id, None).await.unwrap();
        assert_eq!(retrieved.id, party.id);
        assert_eq!(retrieved.email, Some("john@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_mock_port_find_by_email() {
        let port = MockPartyPort::new();

        port.create_individual(
            create_test_individual(),
            Some("john@example.com".to_string()),
            None,
            None,
        ).await.unwrap();

        let found = port.find_by_email("john@example.com", None).await.unwrap();
        assert!(found.is_some());

        let not_found = port.find_by_email("other@example.com", None).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_mock_port_not_found() {
        let port = MockPartyPort::new();
        let result = port.get_party(PartyId::new_v7(), None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().is_not_found());
    }

    #[tokio::test]
    async fn test_mock_port_update() {
        let port = MockPartyPort::new();

        let party = port.create_individual(
            create_test_individual(),
            None,
            None,
            None,
        ).await.unwrap();

        let updated = port.update_party(
            party.id,
            UpdatePartyRequest {
                email: Some("new@example.com".to_string()),
                kyc_status: Some(KycStatus::Verified),
                ..Default::default()
            },
            None,
        ).await.unwrap();

        assert_eq!(updated.email, Some("new@example.com".to_string()));
        assert_eq!(updated.kyc_status, KycStatus::Verified);
    }

    #[tokio::test]
    async fn test_mock_port_deactivate() {
        let port = MockPartyPort::new();

        let party = port.create_individual(
            create_test_individual(),
            None,
            None,
            None,
        ).await.unwrap();

        assert!(party.is_active);

        port.deactivate_party(party.id, None).await.unwrap();

        let retrieved = port.get_party(party.id, None).await.unwrap();
        assert!(!retrieved.is_active);
    }

    #[tokio::test]
    async fn test_mock_port_health_check() {
        let port = MockPartyPort::new();
        let result = port.health_check().await;
        assert_eq!(result.status, core_kernel::AdapterHealth::Healthy);
    }
}
