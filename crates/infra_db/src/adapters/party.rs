//! PostgreSQL Party Adapter
//!
//! This module provides the internal (database) adapter for the party domain,
//! implementing the `PartyPort` trait using PostgreSQL via the `PartyRepository`.
//!
//! # Overview
//!
//! The `PostgresPartyAdapter` serves as the bridge between the domain layer's
//! port interface and the database layer. It:
//!
//! - Translates domain requests into repository operations
//! - Converts database row types back to domain models
//! - Handles error translation between database and port errors
//!
//! # Example
//!
//! ```rust,ignore
//! use infra_db::adapters::PostgresPartyAdapter;
//! use domain_party::{PartyPort, PartyPortExt};
//! use std::sync::Arc;
//!
//! // Create the adapter with a database pool
//! let adapter = PostgresPartyAdapter::new(pool);
//!
//! // Use it through the port trait
//! let port: Arc<dyn PartyPort> = Arc::new(adapter);
//! let party = port.get_party(party_id, None).await?;
//! ```

use async_trait::async_trait;
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::{debug, instrument};

use core_kernel::{
    PartyId, PortError, DomainPort, OperationMetadata,
    HealthCheckable, HealthCheckResult, AdapterHealth,
};
use domain_party::{
    Party, PartyComposition, PartyType, Individual, Corporate, Gender,
    PartyMember, MemberRole, JointDetails, JointType,
    TrustDetails, TrustType, PartnershipDetails, PartnershipType,
    PartyPort, PartyQuery, CreatePartyRequest, CreateMemberRequest, UpdatePartyRequest,
    KycStatus,
};

use crate::repositories::party::{
    PartyRepository, PartyRow, PartyMemberRow, PartyWithComposition,
    NewParty, NewPartyMember, NewJointDetails, NewTrustDetails, NewPartnershipDetails,
    PartyType as DbPartyType, PartyComposition as DbPartyComposition,
    KycStatus as DbKycStatus, MemberRole as DbMemberRole,
    JointType as DbJointType, TrustType as DbTrustType, PartnershipType as DbPartnershipType,
    JointDetailsRow, TrustDetailsRow, PartnershipDetailsRow,
};
use crate::error::DatabaseError;

/// PostgreSQL-backed implementation of the PartyPort trait
///
/// This adapter uses the `PartyRepository` for all database operations
/// and provides the standard internal (database) implementation of the
/// party domain port.
///
/// # Health Checking
///
/// The adapter implements `HealthCheckable` to verify database connectivity.
/// Health checks perform a simple query to ensure the connection pool is
/// operational.
///
/// # Error Handling
///
/// Database errors are translated to `PortError` variants:
/// - `DatabaseError::NotFound` -> `PortError::NotFound`
/// - `DatabaseError::Conflict` -> `PortError::Conflict`
/// - Other errors -> `PortError::Internal`
#[derive(Debug, Clone)]
pub struct PostgresPartyAdapter {
    repository: PartyRepository,
    pool: PgPool,
}

impl PostgresPartyAdapter {
    /// Creates a new PostgreSQL party adapter
    ///
    /// # Arguments
    ///
    /// * `pool` - The PostgreSQL connection pool
    ///
    /// # Returns
    ///
    /// A new adapter instance
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: PartyRepository::new(pool.clone()),
            pool,
        }
    }

    /// Returns a reference to the underlying repository
    ///
    /// This is useful for advanced operations that aren't exposed through
    /// the port trait, such as direct SQL queries or bulk operations.
    pub fn repository(&self) -> &PartyRepository {
        &self.repository
    }
}

// Mark as a domain port
impl DomainPort for PostgresPartyAdapter {}

#[async_trait]
impl HealthCheckable for PostgresPartyAdapter {
    /// Checks database connectivity
    ///
    /// Performs a simple SELECT 1 query to verify the connection pool
    /// is operational and the database is responsive.
    async fn health_check(&self) -> HealthCheckResult {
        let start = std::time::Instant::now();

        let result = sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(_) => HealthCheckResult {
                adapter_id: "postgres-party-adapter".to_string(),
                status: AdapterHealth::Healthy,
                latency_ms,
                message: None,
                checked_at: Utc::now(),
            },
            Err(e) => HealthCheckResult {
                adapter_id: "postgres-party-adapter".to_string(),
                status: AdapterHealth::Unhealthy,
                latency_ms,
                message: Some(format!("Database error: {}", e)),
                checked_at: Utc::now(),
            },
        }
    }
}

#[async_trait]
impl PartyPort for PostgresPartyAdapter {
    #[instrument(skip(self, metadata), fields(party_id = %id))]
    async fn get_party(
        &self,
        id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError> {
        debug!("Fetching party by ID");

        let party_with_comp = self.repository
            .get_with_composition(id.into())
            .await
            .map_err(db_to_port_error)?;

        row_to_party(party_with_comp)
    }

    #[instrument(skip(self, metadata), fields(count = ids.len()))]
    async fn get_parties(
        &self,
        ids: Vec<PartyId>,
        metadata: Option<OperationMetadata>,
    ) -> Result<Vec<Party>, PortError> {
        debug!("Fetching multiple parties");

        let mut parties = Vec::with_capacity(ids.len());
        for id in ids {
            match self.get_party(id, metadata.clone()).await {
                Ok(party) => parties.push(party),
                Err(PortError::NotFound { .. }) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(parties)
    }

    #[instrument(skip(self, metadata))]
    async fn find_parties(
        &self,
        query: PartyQuery,
        _metadata: Option<OperationMetadata>,
    ) -> Result<Vec<Party>, PortError> {
        debug!("Finding parties with query: {:?}", query);

        // Use email search if specified
        let rows = if let Some(ref email) = query.email {
            self.repository.find_by_email(email).await.map_err(db_to_port_error)?
        } else {
            // For now, we don't have a generic find method, so return empty
            // In a full implementation, this would support all query parameters
            Vec::new()
        };

        // Convert rows to parties (simplified - doesn't fetch composition details)
        let mut parties = Vec::with_capacity(rows.len());
        for row in rows {
            let party_with_comp = self.repository
                .get_with_composition(row.party_id)
                .await
                .map_err(db_to_port_error)?;
            parties.push(row_to_party(party_with_comp)?);
        }

        // Apply filters
        let filtered: Vec<_> = parties.into_iter()
            .filter(|p| {
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
            .collect();

        // Apply pagination
        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(100) as usize;

        Ok(filtered.into_iter().skip(offset).take(limit).collect())
    }

    #[instrument(skip(self, request, metadata))]
    async fn create_party(
        &self,
        request: CreatePartyRequest,
        _metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError> {
        debug!("Creating party with composition: {:?}", request.composition);

        match request.composition {
            PartyComposition::Individual => {
                let individual = request.individual
                    .ok_or_else(|| PortError::validation("Individual details required"))?;

                let new_party = NewParty {
                    party_type: DbPartyType::Individual,
                    composition: DbPartyComposition::Individual,
                    first_name: Some(individual.first_name.clone()),
                    last_name: Some(individual.last_name.clone()),
                    company_name: None,
                    email: request.email,
                    phone: request.phone,
                    date_of_birth: Some(individual.date_of_birth),
                    tax_id: individual.tax_id.clone(),
                    kyc_status: DbKycStatus::Pending,
                };

                let row = self.repository.insert(new_party).await.map_err(db_to_port_error)?;

                let party_with_comp = self.repository
                    .get_with_composition(row.party_id)
                    .await
                    .map_err(db_to_port_error)?;

                row_to_party(party_with_comp)
            }

            PartyComposition::Corporate => {
                let corporate = request.corporate
                    .ok_or_else(|| PortError::validation("Corporate details required"))?;

                let new_party = NewParty {
                    party_type: DbPartyType::Corporate,
                    composition: DbPartyComposition::Corporate,
                    first_name: None,
                    last_name: None,
                    company_name: Some(corporate.company_name.clone()),
                    email: request.email,
                    phone: request.phone,
                    date_of_birth: corporate.incorporation_date,
                    tax_id: corporate.tax_id.clone(),
                    kyc_status: DbKycStatus::Pending,
                };

                let row = self.repository.insert(new_party).await.map_err(db_to_port_error)?;

                let party_with_comp = self.repository
                    .get_with_composition(row.party_id)
                    .await
                    .map_err(db_to_port_error)?;

                row_to_party(party_with_comp)
            }

            PartyComposition::Joint => {
                let joint_details = request.joint_details
                    .ok_or_else(|| PortError::validation("Joint details required"))?;

                let new_party = NewParty {
                    party_type: DbPartyType::Joint,
                    composition: DbPartyComposition::Joint,
                    first_name: None,
                    last_name: None,
                    company_name: None,
                    email: request.email,
                    phone: request.phone,
                    date_of_birth: None,
                    tax_id: None,
                    kyc_status: DbKycStatus::Pending,
                };

                let new_joint = NewJointDetails {
                    display_name: joint_details.display_name.clone(),
                    joint_type: domain_to_db_joint_type(joint_details.joint_type),
                    notes: joint_details.notes.clone(),
                };

                let members: Vec<NewPartyMember> = request.members.iter().map(|m| {
                    NewPartyMember {
                        member_party_id: m.member_party_id.into(),
                        role: domain_to_db_member_role(m.role),
                        ownership_percentage: m.ownership_percentage,
                        is_primary_contact: m.is_primary_contact,
                    }
                }).collect();

                let party_with_comp = self.repository
                    .insert_joint(new_party, new_joint, members)
                    .await
                    .map_err(db_to_port_error)?;

                row_to_party(party_with_comp)
            }

            PartyComposition::Trust => {
                let trust_details = request.trust_details
                    .ok_or_else(|| PortError::validation("Trust details required"))?;

                let new_party = NewParty {
                    party_type: DbPartyType::Trust,
                    composition: DbPartyComposition::Trust,
                    first_name: None,
                    last_name: None,
                    company_name: None,
                    email: request.email,
                    phone: request.phone,
                    date_of_birth: None,
                    tax_id: None,
                    kyc_status: DbKycStatus::Pending,
                };

                let new_trust = NewTrustDetails {
                    trust_name: trust_details.trust_name.clone(),
                    trust_id: trust_details.trust_identification.clone(),
                    established_date: trust_details.established_date,
                    trust_type: domain_to_db_trust_type(trust_details.trust_type),
                    is_revocable: trust_details.is_revocable,
                    governing_jurisdiction: trust_details.governing_jurisdiction.clone(),
                };

                let members: Vec<NewPartyMember> = request.members.iter().map(|m| {
                    NewPartyMember {
                        member_party_id: m.member_party_id.into(),
                        role: domain_to_db_member_role(m.role),
                        ownership_percentage: m.ownership_percentage,
                        is_primary_contact: m.is_primary_contact,
                    }
                }).collect();

                let party_with_comp = self.repository
                    .insert_trust(new_party, new_trust, members)
                    .await
                    .map_err(db_to_port_error)?;

                row_to_party(party_with_comp)
            }

            PartyComposition::Partnership => {
                let partnership_details = request.partnership_details
                    .ok_or_else(|| PortError::validation("Partnership details required"))?;

                let new_party = NewParty {
                    party_type: DbPartyType::Partnership,
                    composition: DbPartyComposition::Partnership,
                    first_name: None,
                    last_name: None,
                    company_name: None,
                    email: request.email,
                    phone: request.phone,
                    date_of_birth: None,
                    tax_id: partnership_details.tax_id.clone(),
                    kyc_status: DbKycStatus::Pending,
                };

                let new_partnership = NewPartnershipDetails {
                    partnership_name: partnership_details.partnership_name.clone(),
                    registration_number: partnership_details.registration_number.clone(),
                    tax_id: partnership_details.tax_id.clone(),
                    partnership_type: domain_to_db_partnership_type(partnership_details.partnership_type),
                    formation_date: partnership_details.formation_date,
                    formation_jurisdiction: partnership_details.formation_jurisdiction.clone(),
                };

                let members: Vec<NewPartyMember> = request.members.iter().map(|m| {
                    NewPartyMember {
                        member_party_id: m.member_party_id.into(),
                        role: domain_to_db_member_role(m.role),
                        ownership_percentage: m.ownership_percentage,
                        is_primary_contact: m.is_primary_contact,
                    }
                }).collect();

                let party_with_comp = self.repository
                    .insert_partnership(new_party, new_partnership, members)
                    .await
                    .map_err(db_to_port_error)?;

                row_to_party(party_with_comp)
            }
        }
    }

    #[instrument(skip(self, request, metadata), fields(party_id = %id))]
    async fn update_party(
        &self,
        id: PartyId,
        request: UpdatePartyRequest,
        _metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError> {
        debug!("Updating party");

        // For now, we'll re-fetch after update
        // A full implementation would have an update method in the repository
        let party_with_comp = self.repository
            .get_with_composition(id.into())
            .await
            .map_err(db_to_port_error)?;

        // In a full implementation, we would update the party here
        // For now, we just return the existing party
        // This is a placeholder that should be expanded
        row_to_party(party_with_comp)
    }

    #[instrument(skip(self, metadata), fields(party_id = %id))]
    async fn deactivate_party(
        &self,
        id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<(), PortError> {
        debug!("Deactivating party");

        // Verify party exists
        let _ = self.repository
            .get_by_id(id.into())
            .await
            .map_err(db_to_port_error)?;

        // A full implementation would set is_active = false
        // This is a placeholder
        Ok(())
    }

    #[instrument(skip(self, metadata), fields(party_id = %party_id))]
    async fn get_members(
        &self,
        party_id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<Vec<PartyMember>, PortError> {
        debug!("Fetching party members");

        let rows = self.repository
            .get_members(party_id.into())
            .await
            .map_err(db_to_port_error)?;

        Ok(rows.into_iter().map(member_row_to_domain).collect())
    }

    #[instrument(skip(self, request, metadata), fields(party_id = %party_id))]
    async fn add_member(
        &self,
        party_id: PartyId,
        request: CreateMemberRequest,
        _metadata: Option<OperationMetadata>,
    ) -> Result<PartyMember, PortError> {
        debug!("Adding member to party");

        let new_member = NewPartyMember {
            member_party_id: request.member_party_id.into(),
            role: domain_to_db_member_role(request.role),
            ownership_percentage: request.ownership_percentage,
            is_primary_contact: request.is_primary_contact,
        };

        let row = self.repository
            .add_member(party_id.into(), new_member)
            .await
            .map_err(db_to_port_error)?;

        Ok(member_row_to_domain(row))
    }

    #[instrument(skip(self, metadata), fields(party_id = %party_id, member_party_id = %member_party_id))]
    async fn remove_member(
        &self,
        party_id: PartyId,
        member_party_id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<(), PortError> {
        debug!("Removing member from party");

        let affected = self.repository
            .remove_member(party_id.into(), member_party_id.into())
            .await
            .map_err(db_to_port_error)?;

        if affected == 0 {
            return Err(PortError::not_found("Member", member_party_id));
        }

        Ok(())
    }

    #[instrument(skip(self, metadata), fields(party_id = %party_id, member_party_id = %member_party_id))]
    async fn update_member_ownership(
        &self,
        party_id: PartyId,
        member_party_id: PartyId,
        new_percentage: Decimal,
        _metadata: Option<OperationMetadata>,
    ) -> Result<PartyMember, PortError> {
        debug!("Updating member ownership to {}%", new_percentage);

        // First find the member to get their member_id
        let members = self.repository
            .get_members(party_id.into())
            .await
            .map_err(db_to_port_error)?;

        let member = members.into_iter()
            .find(|m| m.member_party_id == member_party_id.into())
            .ok_or_else(|| PortError::not_found("Member", member_party_id))?;

        let updated = self.repository
            .update_member_ownership(member.member_id, new_percentage)
            .await
            .map_err(db_to_port_error)?;

        Ok(member_row_to_domain(updated))
    }

    #[instrument(skip(self, metadata), fields(party_id = %party_id, member_party_id = %member_party_id))]
    async fn set_primary_contact(
        &self,
        party_id: PartyId,
        member_party_id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<(), PortError> {
        debug!("Setting primary contact");

        self.repository
            .set_primary_contact(party_id.into(), member_party_id.into())
            .await
            .map_err(db_to_port_error)?;

        Ok(())
    }

    #[instrument(skip(self, metadata), fields(member_party_id = %member_party_id))]
    async fn find_by_member(
        &self,
        member_party_id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<Vec<Party>, PortError> {
        debug!("Finding parties by member");

        let rows = self.repository
            .find_by_member(member_party_id.into())
            .await
            .map_err(db_to_port_error)?;

        let mut parties = Vec::with_capacity(rows.len());
        for row in rows {
            let party_with_comp = self.repository
                .get_with_composition(row.party_id)
                .await
                .map_err(db_to_port_error)?;
            parties.push(row_to_party(party_with_comp)?);
        }

        Ok(parties)
    }

    #[instrument(skip(self, metadata), fields(party_id = %id))]
    async fn exists(
        &self,
        id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<bool, PortError> {
        debug!("Checking if party exists");

        match self.repository.get_by_id(id.into()).await {
            Ok(_) => Ok(true),
            Err(DatabaseError::NotFound { .. }) => Ok(false),
            Err(e) => Err(db_to_port_error(e)),
        }
    }

    #[instrument(skip(self, metadata), fields(party_id = %id))]
    async fn update_kyc_status(
        &self,
        id: PartyId,
        status: KycStatus,
        _metadata: Option<OperationMetadata>,
    ) -> Result<(), PortError> {
        debug!("Updating KYC status to {:?}", status);

        // Verify party exists
        let _ = self.repository
            .get_by_id(id.into())
            .await
            .map_err(db_to_port_error)?;

        // A full implementation would update the KYC status
        // This is a placeholder
        Ok(())
    }
}

// =============================================================================
// Conversion Functions
// =============================================================================

/// Converts a database error to a port error
fn db_to_port_error(e: DatabaseError) -> PortError {
    match e {
        DatabaseError::NotFound { entity, id } => PortError::NotFound {
            entity: entity.to_string(),
            id: id.to_string(),
        },
        DatabaseError::Conflict(msg) => PortError::Conflict(msg),
        DatabaseError::Connection(msg) => PortError::Connection(msg),
        _ => PortError::Internal(e.to_string()),
    }
}

/// Converts a database party row with composition to a domain Party
fn row_to_party(data: PartyWithComposition) -> Result<Party, PortError> {
    let row = &data.party;

    let composition = db_to_domain_composition(row.composition);
    let party_type = db_to_domain_party_type(row.party_type);

    let mut party = Party {
        id: PartyId::from(row.party_id),
        party_type,
        composition,
        individual: None,
        corporate: None,
        joint_details: None,
        trust_details: None,
        partnership_details: None,
        members: Vec::new(),
        email: row.email.clone(),
        phone: row.phone.clone(),
        kyc_status: db_to_domain_kyc_status(row.kyc_status),
        is_active: true,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    // Fill in composition-specific details
    match composition {
        PartyComposition::Individual => {
            party.individual = Some(Individual {
                first_name: row.first_name.clone().unwrap_or_default(),
                middle_name: None,
                last_name: row.last_name.clone().unwrap_or_default(),
                date_of_birth: row.date_of_birth.unwrap_or_default(),
                gender: None,
                nationality: None,
                tax_id: row.tax_id.clone(),
                occupation: None,
            });
        }
        PartyComposition::Corporate => {
            party.corporate = Some(Corporate {
                company_name: row.company_name.clone().unwrap_or_default(),
                registration_number: None,
                tax_id: row.tax_id.clone(),
                industry: None,
                incorporation_date: row.date_of_birth,
                incorporation_jurisdiction: None,
                corporate_type: None,
            });
        }
        PartyComposition::Joint => {
            if let Some(ref joint) = data.joint_details {
                party.joint_details = Some(JointDetails {
                    display_name: joint.display_name.clone(),
                    joint_type: db_to_domain_joint_type(joint.joint_type),
                    notes: joint.notes.clone(),
                });
            }
        }
        PartyComposition::Trust => {
            if let Some(ref trust) = data.trust_details {
                party.trust_details = Some(TrustDetails {
                    trust_name: trust.trust_name.clone(),
                    trust_identification: trust.trust_identification.clone(),
                    established_date: trust.established_date,
                    trust_type: db_to_domain_trust_type(trust.trust_type),
                    is_revocable: trust.is_revocable,
                    governing_jurisdiction: trust.governing_jurisdiction.clone(),
                });
            }
        }
        PartyComposition::Partnership => {
            if let Some(ref partnership) = data.partnership_details {
                party.partnership_details = Some(PartnershipDetails {
                    partnership_name: partnership.partnership_name.clone(),
                    registration_number: partnership.registration_number.clone(),
                    tax_id: partnership.tax_id.clone(),
                    partnership_type: db_to_domain_partnership_type(partnership.partnership_type),
                    formation_date: partnership.formation_date,
                    formation_jurisdiction: partnership.formation_jurisdiction.clone(),
                });
            }
        }
    }

    // Convert members
    party.members = data.members.into_iter()
        .map(member_row_to_domain)
        .collect();

    Ok(party)
}

/// Converts a member database row to a domain PartyMember
fn member_row_to_domain(row: PartyMemberRow) -> PartyMember {
    PartyMember {
        member_party_id: PartyId::from(row.member_party_id),
        role: db_to_domain_member_role(row.role),
        ownership_percentage: row.ownership_percentage,
        is_primary_contact: row.is_primary_contact,
        effective_from: row.effective_from,
        effective_to: row.effective_to,
    }
}

// =============================================================================
// Type Mapping Functions: Database -> Domain
// =============================================================================

fn db_to_domain_composition(c: DbPartyComposition) -> PartyComposition {
    match c {
        DbPartyComposition::Individual => PartyComposition::Individual,
        DbPartyComposition::Corporate => PartyComposition::Corporate,
        DbPartyComposition::Joint => PartyComposition::Joint,
        DbPartyComposition::Trust => PartyComposition::Trust,
        DbPartyComposition::Partnership => PartyComposition::Partnership,
    }
}

fn db_to_domain_party_type(t: DbPartyType) -> PartyType {
    match t {
        DbPartyType::Individual => PartyType::Individual,
        DbPartyType::Corporate => PartyType::Corporate,
        DbPartyType::Agent => PartyType::Agent,
        DbPartyType::Broker => PartyType::Broker,
        DbPartyType::Beneficiary => PartyType::Individual, // Map legacy Beneficiary to Individual
        DbPartyType::Joint => PartyType::Joint,
        DbPartyType::Trust => PartyType::Trust,
        DbPartyType::Partnership => PartyType::Partnership,
    }
}

fn db_to_domain_kyc_status(s: DbKycStatus) -> KycStatus {
    match s {
        DbKycStatus::Pending => KycStatus::Pending,
        DbKycStatus::InProgress => KycStatus::InProgress,
        DbKycStatus::Verified => KycStatus::Verified,
        DbKycStatus::Failed => KycStatus::Failed,
        DbKycStatus::Expired => KycStatus::Expired,
    }
}

fn db_to_domain_member_role(r: DbMemberRole) -> MemberRole {
    match r {
        DbMemberRole::PrimaryOwner => MemberRole::PrimaryOwner,
        DbMemberRole::CoOwner => MemberRole::CoOwner,
        DbMemberRole::Trustee => MemberRole::Trustee,
        DbMemberRole::TrustBeneficiary => MemberRole::TrustBeneficiary,
        DbMemberRole::Settlor => MemberRole::Settlor,
        DbMemberRole::ManagingPartner => MemberRole::ManagingPartner,
        DbMemberRole::Partner => MemberRole::Partner,
        DbMemberRole::SilentPartner => MemberRole::SilentPartner,
        DbMemberRole::AuthorizedSignatory => MemberRole::AuthorizedSignatory,
        DbMemberRole::Director => MemberRole::Director,
    }
}

fn db_to_domain_joint_type(t: DbJointType) -> JointType {
    match t {
        DbJointType::JointTenants => JointType::JointTenants,
        DbJointType::TenantsInCommon => JointType::TenantsInCommon,
        DbJointType::CommunityProperty => JointType::CommunityProperty,
        DbJointType::Other => JointType::Other(String::new()),
    }
}

fn db_to_domain_trust_type(t: DbTrustType) -> TrustType {
    match t {
        DbTrustType::RevocableLiving => TrustType::RevocableLiving,
        DbTrustType::Ilit => TrustType::ILIT,
        DbTrustType::CharitableRemainder => TrustType::CharitableRemainder,
        DbTrustType::SpecialNeeds => TrustType::SpecialNeeds,
        DbTrustType::Testamentary => TrustType::Testamentary,
        DbTrustType::Other => TrustType::Other(String::new()),
    }
}

fn db_to_domain_partnership_type(t: DbPartnershipType) -> PartnershipType {
    match t {
        DbPartnershipType::GeneralPartnership => PartnershipType::GeneralPartnership,
        DbPartnershipType::LimitedPartnership => PartnershipType::LimitedPartnership,
        DbPartnershipType::Llp => PartnershipType::LLP,
        DbPartnershipType::Other => PartnershipType::Other(String::new()),
    }
}

// =============================================================================
// Type Mapping Functions: Domain -> Database
// =============================================================================

fn domain_to_db_member_role(r: MemberRole) -> DbMemberRole {
    match r {
        MemberRole::PrimaryOwner => DbMemberRole::PrimaryOwner,
        MemberRole::CoOwner => DbMemberRole::CoOwner,
        MemberRole::Trustee => DbMemberRole::Trustee,
        MemberRole::TrustBeneficiary => DbMemberRole::TrustBeneficiary,
        MemberRole::Settlor => DbMemberRole::Settlor,
        MemberRole::ManagingPartner => DbMemberRole::ManagingPartner,
        MemberRole::Partner => DbMemberRole::Partner,
        MemberRole::SilentPartner => DbMemberRole::SilentPartner,
        MemberRole::AuthorizedSignatory => DbMemberRole::AuthorizedSignatory,
        MemberRole::Director => DbMemberRole::Director,
    }
}

fn domain_to_db_joint_type(t: JointType) -> DbJointType {
    match t {
        JointType::JointTenants => DbJointType::JointTenants,
        JointType::TenantsInCommon => DbJointType::TenantsInCommon,
        JointType::CommunityProperty => DbJointType::CommunityProperty,
        JointType::Other(_) => DbJointType::Other,
    }
}

fn domain_to_db_trust_type(t: TrustType) -> DbTrustType {
    match t {
        TrustType::RevocableLiving => DbTrustType::RevocableLiving,
        TrustType::ILIT => DbTrustType::Ilit,
        TrustType::CharitableRemainder => DbTrustType::CharitableRemainder,
        TrustType::SpecialNeeds => DbTrustType::SpecialNeeds,
        TrustType::Testamentary => DbTrustType::Testamentary,
        TrustType::Other(_) => DbTrustType::Other,
    }
}

fn domain_to_db_partnership_type(t: PartnershipType) -> DbPartnershipType {
    match t {
        PartnershipType::GeneralPartnership => DbPartnershipType::GeneralPartnership,
        PartnershipType::LimitedPartnership => DbPartnershipType::LimitedPartnership,
        PartnershipType::LLP => DbPartnershipType::Llp,
        PartnershipType::Other(_) => DbPartnershipType::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composition_conversion_roundtrip() {
        let compositions = [
            PartyComposition::Individual,
            PartyComposition::Corporate,
            PartyComposition::Joint,
            PartyComposition::Trust,
            PartyComposition::Partnership,
        ];

        for comp in compositions {
            let db_comp = match comp {
                PartyComposition::Individual => DbPartyComposition::Individual,
                PartyComposition::Corporate => DbPartyComposition::Corporate,
                PartyComposition::Joint => DbPartyComposition::Joint,
                PartyComposition::Trust => DbPartyComposition::Trust,
                PartyComposition::Partnership => DbPartyComposition::Partnership,
            };
            let domain_comp = db_to_domain_composition(db_comp);
            assert_eq!(domain_comp, comp);
        }
    }

    #[test]
    fn test_member_role_conversion_roundtrip() {
        let roles = [
            MemberRole::PrimaryOwner,
            MemberRole::CoOwner,
            MemberRole::Trustee,
            MemberRole::ManagingPartner,
            MemberRole::Partner,
        ];

        for role in roles {
            let db_role = domain_to_db_member_role(role);
            let domain_role = db_to_domain_member_role(db_role);
            assert_eq!(domain_role, role);
        }
    }
}
