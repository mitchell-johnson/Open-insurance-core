//! Party repository implementation
//!
//! This module provides database access for party (customer, agent) data,
//! implementing bi-temporal versioning for party changes.
//!
//! # Party Composition Support
//!
//! The repository supports all party composition types:
//! - Individual: Single person
//! - Corporate: Company/legal entity
//! - Joint: Multiple individuals with shared ownership
//! - Trust: Trust entity with trustees
//! - Partnership: Business partnership with partners
//!
//! Composite parties (Joint, Trust, Partnership) store their members in the
//! `party_members` table, with bi-temporal support for tracking membership
//! changes over time.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DatabaseError;

/// Repository for managing party data with bi-temporal support
///
/// The PartyRepository handles all database operations for parties,
/// including customers, agents, and other stakeholders. It supports
/// composite party structures like joint owners, trusts, and partnerships.
#[derive(Debug, Clone)]
pub struct PartyRepository {
    pool: PgPool,
}

impl PartyRepository {
    /// Creates a new PartyRepository with the given connection pool
    ///
    /// # Arguments
    ///
    /// * `pool` - The PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Retrieves a party by their identifier
    ///
    /// # Arguments
    ///
    /// * `party_id` - The party identifier
    ///
    /// # Returns
    ///
    /// The current party record or NotFound error
    pub async fn get_by_id(&self, party_id: Uuid) -> Result<PartyRow, DatabaseError> {
        let party = sqlx::query_as!(
            PartyRow,
            r#"
            SELECT
                version_id,
                party_id,
                party_type as "party_type: PartyType",
                composition as "composition: PartyComposition",
                first_name,
                last_name,
                company_name,
                email,
                phone,
                date_of_birth,
                tax_id,
                kyc_status as "kyc_status: KycStatus",
                created_at,
                updated_at
            FROM party_versions
            WHERE party_id = $1 AND upper(sys_period) IS NULL
            "#,
            party_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DatabaseError::not_found("Party", party_id))?;

        Ok(party)
    }

    /// Retrieves a party with all composition details
    ///
    /// This method fetches the party along with any joint, trust, or partnership
    /// details, as well as all active members.
    ///
    /// # Arguments
    ///
    /// * `party_id` - The party identifier
    ///
    /// # Returns
    ///
    /// The party with full composition details
    pub async fn get_with_composition(&self, party_id: Uuid) -> Result<PartyWithComposition, DatabaseError> {
        let party = self.get_by_id(party_id).await?;

        // Get composition-specific details
        let joint_details = if party.composition == PartyComposition::Joint {
            self.get_joint_details(party_id).await?
        } else {
            None
        };

        let trust_details = if party.composition == PartyComposition::Trust {
            self.get_trust_details(party_id).await?
        } else {
            None
        };

        let partnership_details = if party.composition == PartyComposition::Partnership {
            self.get_partnership_details(party_id).await?
        } else {
            None
        };

        // Get members for composite parties
        let members = if matches!(
            party.composition,
            PartyComposition::Joint | PartyComposition::Trust | PartyComposition::Partnership
        ) {
            self.get_members(party_id).await?
        } else {
            Vec::new()
        };

        Ok(PartyWithComposition {
            party,
            joint_details,
            trust_details,
            partnership_details,
            members,
        })
    }

    /// Searches for parties by email address
    ///
    /// # Arguments
    ///
    /// * `email` - The email address to search for
    ///
    /// # Returns
    ///
    /// A vector of matching party records
    pub async fn find_by_email(&self, email: &str) -> Result<Vec<PartyRow>, DatabaseError> {
        let parties = sqlx::query_as!(
            PartyRow,
            r#"
            SELECT
                version_id,
                party_id,
                party_type as "party_type: PartyType",
                composition as "composition: PartyComposition",
                first_name,
                last_name,
                company_name,
                email,
                phone,
                date_of_birth,
                tax_id,
                kyc_status as "kyc_status: KycStatus",
                created_at,
                updated_at
            FROM party_versions
            WHERE lower(email) = lower($1) AND upper(sys_period) IS NULL
            "#,
            email
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(parties)
    }

    /// Finds all parties where a given party is a member
    ///
    /// This is useful for finding all joint ownerships, trusts, or partnerships
    /// that a person belongs to.
    ///
    /// # Arguments
    ///
    /// * `member_party_id` - The party ID of the member
    ///
    /// # Returns
    ///
    /// A vector of composite parties containing this member
    pub async fn find_by_member(&self, member_party_id: Uuid) -> Result<Vec<PartyRow>, DatabaseError> {
        let parties = sqlx::query_as!(
            PartyRow,
            r#"
            SELECT DISTINCT
                pv.version_id,
                pv.party_id,
                pv.party_type as "party_type: PartyType",
                pv.composition as "composition: PartyComposition",
                pv.first_name,
                pv.last_name,
                pv.company_name,
                pv.email,
                pv.phone,
                pv.date_of_birth,
                pv.tax_id,
                pv.kyc_status as "kyc_status: KycStatus",
                pv.created_at,
                pv.updated_at
            FROM party_versions pv
            INNER JOIN party_members pm ON pv.party_id = pm.party_id
            WHERE pm.member_party_id = $1
              AND pm.effective_to IS NULL
              AND upper(pm.sys_period) IS NULL
              AND upper(pv.sys_period) IS NULL
            "#,
            member_party_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(parties)
    }

    /// Creates a new party
    ///
    /// # Arguments
    ///
    /// * `party` - The party data to insert
    ///
    /// # Returns
    ///
    /// The created party row with generated identifiers
    pub async fn insert(&self, party: NewParty) -> Result<PartyRow, DatabaseError> {
        let version_id = Uuid::new_v4();
        let party_id = Uuid::new_v4();
        let now = Utc::now();

        let row = sqlx::query_as!(
            PartyRow,
            r#"
            INSERT INTO party_versions (
                version_id,
                party_id,
                party_type,
                composition,
                first_name,
                last_name,
                company_name,
                email,
                phone,
                date_of_birth,
                tax_id,
                kyc_status,
                valid_period,
                sys_period,
                created_at,
                updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                tstzrange($13, NULL),
                tstzrange($13, NULL),
                $13, $13
            )
            RETURNING
                version_id,
                party_id,
                party_type as "party_type: PartyType",
                composition as "composition: PartyComposition",
                first_name,
                last_name,
                company_name,
                email,
                phone,
                date_of_birth,
                tax_id,
                kyc_status as "kyc_status: KycStatus",
                created_at,
                updated_at
            "#,
            version_id,
            party_id,
            party.party_type as PartyType,
            party.composition as PartyComposition,
            party.first_name,
            party.last_name,
            party.company_name,
            party.email,
            party.phone,
            party.date_of_birth,
            party.tax_id,
            party.kyc_status as KycStatus,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Creates a new joint party with members
    ///
    /// # Arguments
    ///
    /// * `party` - The party data
    /// * `joint_details` - Joint ownership details
    /// * `members` - The member parties
    ///
    /// # Returns
    ///
    /// The created party with all details
    pub async fn insert_joint(
        &self,
        party: NewParty,
        joint_details: NewJointDetails,
        members: Vec<NewPartyMember>,
    ) -> Result<PartyWithComposition, DatabaseError> {
        let mut tx = self.pool.begin().await?;

        // Insert the party
        let version_id = Uuid::new_v4();
        let party_id = Uuid::new_v4();
        let now = Utc::now();

        let party_row = sqlx::query_as!(
            PartyRow,
            r#"
            INSERT INTO party_versions (
                version_id, party_id, party_type, composition,
                first_name, last_name, company_name, email, phone,
                date_of_birth, tax_id, kyc_status,
                valid_period, sys_period, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                tstzrange($13, NULL), tstzrange($13, NULL), $13, $13
            )
            RETURNING
                version_id, party_id,
                party_type as "party_type: PartyType",
                composition as "composition: PartyComposition",
                first_name, last_name, company_name, email, phone,
                date_of_birth, tax_id,
                kyc_status as "kyc_status: KycStatus",
                created_at, updated_at
            "#,
            version_id,
            party_id,
            PartyType::Joint as PartyType,
            PartyComposition::Joint as PartyComposition,
            party.first_name,
            party.last_name,
            party.company_name,
            party.email,
            party.phone,
            party.date_of_birth,
            party.tax_id,
            party.kyc_status as KycStatus,
            now
        )
        .fetch_one(&mut *tx)
        .await?;

        // Insert joint details
        let joint_row = sqlx::query_as!(
            JointDetailsRow,
            r#"
            INSERT INTO party_joint_details (
                party_id, display_name, joint_type, notes,
                valid_period, sys_period, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4,
                tstzrange($5, NULL), tstzrange($5, NULL), $5, $5
            )
            RETURNING
                joint_id, party_id, display_name,
                joint_type as "joint_type: JointType",
                notes, created_at, updated_at
            "#,
            party_id,
            joint_details.display_name,
            joint_details.joint_type as JointType,
            joint_details.notes,
            now
        )
        .fetch_one(&mut *tx)
        .await?;

        // Insert members
        let mut member_rows = Vec::with_capacity(members.len());
        for member in members {
            let member_row = sqlx::query_as!(
                PartyMemberRow,
                r#"
                INSERT INTO party_members (
                    party_id, member_party_id, role, ownership_percentage,
                    is_primary_contact, effective_from, effective_to,
                    sys_period, created_at, updated_at
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7,
                    tstzrange($6, NULL), $6, $6
                )
                RETURNING
                    member_id, party_id, member_party_id,
                    role as "role: MemberRole",
                    ownership_percentage,
                    is_primary_contact, effective_from, effective_to,
                    created_at, updated_at
                "#,
                party_id,
                member.member_party_id,
                member.role as MemberRole,
                member.ownership_percentage,
                member.is_primary_contact,
                now,
                None::<DateTime<Utc>>
            )
            .fetch_one(&mut *tx)
            .await?;
            member_rows.push(member_row);
        }

        tx.commit().await?;

        Ok(PartyWithComposition {
            party: party_row,
            joint_details: Some(joint_row),
            trust_details: None,
            partnership_details: None,
            members: member_rows,
        })
    }

    /// Creates a new trust party with trustees
    ///
    /// # Arguments
    ///
    /// * `party` - The party data
    /// * `trust_details` - Trust details
    /// * `members` - The trustee and other members
    ///
    /// # Returns
    ///
    /// The created party with all details
    pub async fn insert_trust(
        &self,
        party: NewParty,
        trust_details: NewTrustDetails,
        members: Vec<NewPartyMember>,
    ) -> Result<PartyWithComposition, DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let version_id = Uuid::new_v4();
        let party_id = Uuid::new_v4();
        let now = Utc::now();

        let party_row = sqlx::query_as!(
            PartyRow,
            r#"
            INSERT INTO party_versions (
                version_id, party_id, party_type, composition,
                first_name, last_name, company_name, email, phone,
                date_of_birth, tax_id, kyc_status,
                valid_period, sys_period, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                tstzrange($13, NULL), tstzrange($13, NULL), $13, $13
            )
            RETURNING
                version_id, party_id,
                party_type as "party_type: PartyType",
                composition as "composition: PartyComposition",
                first_name, last_name, company_name, email, phone,
                date_of_birth, tax_id,
                kyc_status as "kyc_status: KycStatus",
                created_at, updated_at
            "#,
            version_id,
            party_id,
            PartyType::Trust as PartyType,
            PartyComposition::Trust as PartyComposition,
            party.first_name,
            party.last_name,
            party.company_name,
            party.email,
            party.phone,
            party.date_of_birth,
            party.tax_id,
            party.kyc_status as KycStatus,
            now
        )
        .fetch_one(&mut *tx)
        .await?;

        // Insert trust details
        let trust_row = sqlx::query_as!(
            TrustDetailsRow,
            r#"
            INSERT INTO party_trust_details (
                party_id, trust_name, trust_identification, established_date,
                trust_type, is_revocable, governing_jurisdiction,
                valid_period, sys_period, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                tstzrange($8, NULL), tstzrange($8, NULL), $8, $8
            )
            RETURNING
                trust_id, party_id, trust_name, trust_identification,
                established_date,
                trust_type as "trust_type: TrustType",
                is_revocable, governing_jurisdiction,
                created_at, updated_at
            "#,
            party_id,
            trust_details.trust_name,
            trust_details.trust_id,
            trust_details.established_date,
            trust_details.trust_type as TrustType,
            trust_details.is_revocable,
            trust_details.governing_jurisdiction,
            now
        )
        .fetch_one(&mut *tx)
        .await?;

        // Insert members
        let mut member_rows = Vec::with_capacity(members.len());
        for member in members {
            let member_row = sqlx::query_as!(
                PartyMemberRow,
                r#"
                INSERT INTO party_members (
                    party_id, member_party_id, role, ownership_percentage,
                    is_primary_contact, effective_from, effective_to,
                    sys_period, created_at, updated_at
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7,
                    tstzrange($6, NULL), $6, $6
                )
                RETURNING
                    member_id, party_id, member_party_id,
                    role as "role: MemberRole",
                    ownership_percentage,
                    is_primary_contact, effective_from, effective_to,
                    created_at, updated_at
                "#,
                party_id,
                member.member_party_id,
                member.role as MemberRole,
                member.ownership_percentage,
                member.is_primary_contact,
                now,
                None::<DateTime<Utc>>
            )
            .fetch_one(&mut *tx)
            .await?;
            member_rows.push(member_row);
        }

        tx.commit().await?;

        Ok(PartyWithComposition {
            party: party_row,
            joint_details: None,
            trust_details: Some(trust_row),
            partnership_details: None,
            members: member_rows,
        })
    }

    /// Creates a new partnership party with partners
    ///
    /// # Arguments
    ///
    /// * `party` - The party data
    /// * `partnership_details` - Partnership details
    /// * `members` - The partner members
    ///
    /// # Returns
    ///
    /// The created party with all details
    pub async fn insert_partnership(
        &self,
        party: NewParty,
        partnership_details: NewPartnershipDetails,
        members: Vec<NewPartyMember>,
    ) -> Result<PartyWithComposition, DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let version_id = Uuid::new_v4();
        let party_id = Uuid::new_v4();
        let now = Utc::now();

        let party_row = sqlx::query_as!(
            PartyRow,
            r#"
            INSERT INTO party_versions (
                version_id, party_id, party_type, composition,
                first_name, last_name, company_name, email, phone,
                date_of_birth, tax_id, kyc_status,
                valid_period, sys_period, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                tstzrange($13, NULL), tstzrange($13, NULL), $13, $13
            )
            RETURNING
                version_id, party_id,
                party_type as "party_type: PartyType",
                composition as "composition: PartyComposition",
                first_name, last_name, company_name, email, phone,
                date_of_birth, tax_id,
                kyc_status as "kyc_status: KycStatus",
                created_at, updated_at
            "#,
            version_id,
            party_id,
            PartyType::Partnership as PartyType,
            PartyComposition::Partnership as PartyComposition,
            party.first_name,
            party.last_name,
            party.company_name,
            party.email,
            party.phone,
            party.date_of_birth,
            party.tax_id,
            party.kyc_status as KycStatus,
            now
        )
        .fetch_one(&mut *tx)
        .await?;

        // Insert partnership details
        let partnership_row = sqlx::query_as!(
            PartnershipDetailsRow,
            r#"
            INSERT INTO party_partnership_details (
                party_id, partnership_name, registration_number, tax_id,
                partnership_type, formation_date, formation_jurisdiction,
                valid_period, sys_period, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                tstzrange($8, NULL), tstzrange($8, NULL), $8, $8
            )
            RETURNING
                partnership_id, party_id, partnership_name, registration_number,
                tax_id, partnership_type as "partnership_type: PartnershipType",
                formation_date, formation_jurisdiction,
                created_at, updated_at
            "#,
            party_id,
            partnership_details.partnership_name,
            partnership_details.registration_number,
            partnership_details.tax_id,
            partnership_details.partnership_type as PartnershipType,
            partnership_details.formation_date,
            partnership_details.formation_jurisdiction,
            now
        )
        .fetch_one(&mut *tx)
        .await?;

        // Insert members
        let mut member_rows = Vec::with_capacity(members.len());
        for member in members {
            let member_row = sqlx::query_as!(
                PartyMemberRow,
                r#"
                INSERT INTO party_members (
                    party_id, member_party_id, role, ownership_percentage,
                    is_primary_contact, effective_from, effective_to,
                    sys_period, created_at, updated_at
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7,
                    tstzrange($6, NULL), $6, $6
                )
                RETURNING
                    member_id, party_id, member_party_id,
                    role as "role: MemberRole",
                    ownership_percentage,
                    is_primary_contact, effective_from, effective_to,
                    created_at, updated_at
                "#,
                party_id,
                member.member_party_id,
                member.role as MemberRole,
                member.ownership_percentage,
                member.is_primary_contact,
                now,
                None::<DateTime<Utc>>
            )
            .fetch_one(&mut *tx)
            .await?;
            member_rows.push(member_row);
        }

        tx.commit().await?;

        Ok(PartyWithComposition {
            party: party_row,
            joint_details: None,
            trust_details: None,
            partnership_details: Some(partnership_row),
            members: member_rows,
        })
    }

    /// Adds a member to a composite party
    ///
    /// # Arguments
    ///
    /// * `party_id` - The composite party ID
    /// * `member` - The member to add
    ///
    /// # Returns
    ///
    /// The created member record
    pub async fn add_member(
        &self,
        party_id: Uuid,
        member: NewPartyMember,
    ) -> Result<PartyMemberRow, DatabaseError> {
        let now = Utc::now();

        let row = sqlx::query_as!(
            PartyMemberRow,
            r#"
            INSERT INTO party_members (
                party_id, member_party_id, role, ownership_percentage,
                is_primary_contact, effective_from, effective_to,
                sys_period, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                tstzrange($6, NULL), $6, $6
            )
            RETURNING
                member_id, party_id, member_party_id,
                role as "role: MemberRole",
                ownership_percentage,
                is_primary_contact, effective_from, effective_to,
                created_at, updated_at
            "#,
            party_id,
            member.member_party_id,
            member.role as MemberRole,
            member.ownership_percentage,
            member.is_primary_contact,
            now,
            None::<DateTime<Utc>>
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Removes a member from a composite party (soft delete)
    ///
    /// Sets the effective_to date to now, keeping the historical record.
    ///
    /// # Arguments
    ///
    /// * `party_id` - The composite party ID
    /// * `member_party_id` - The member party ID to remove
    ///
    /// # Returns
    ///
    /// The number of members removed
    pub async fn remove_member(
        &self,
        party_id: Uuid,
        member_party_id: Uuid,
    ) -> Result<u64, DatabaseError> {
        let now = Utc::now();

        let result = sqlx::query!(
            r#"
            UPDATE party_members
            SET effective_to = $3, updated_at = $3
            WHERE party_id = $1
              AND member_party_id = $2
              AND effective_to IS NULL
              AND upper(sys_period) IS NULL
            "#,
            party_id,
            member_party_id,
            now
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Gets all active members of a composite party
    ///
    /// # Arguments
    ///
    /// * `party_id` - The composite party ID
    ///
    /// # Returns
    ///
    /// A vector of active member records
    pub async fn get_members(&self, party_id: Uuid) -> Result<Vec<PartyMemberRow>, DatabaseError> {
        let members = sqlx::query_as!(
            PartyMemberRow,
            r#"
            SELECT
                member_id,
                party_id,
                member_party_id,
                role as "role: MemberRole",
                ownership_percentage,
                is_primary_contact,
                effective_from,
                effective_to,
                created_at,
                updated_at
            FROM party_members
            WHERE party_id = $1
              AND effective_to IS NULL
              AND upper(sys_period) IS NULL
            ORDER BY is_primary_contact DESC, created_at ASC
            "#,
            party_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(members)
    }

    /// Gets joint details for a party
    async fn get_joint_details(&self, party_id: Uuid) -> Result<Option<JointDetailsRow>, DatabaseError> {
        let details = sqlx::query_as!(
            JointDetailsRow,
            r#"
            SELECT
                joint_id,
                party_id,
                display_name,
                joint_type as "joint_type: JointType",
                notes,
                created_at,
                updated_at
            FROM party_joint_details
            WHERE party_id = $1 AND upper(sys_period) IS NULL
            "#,
            party_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(details)
    }

    /// Gets trust details for a party
    async fn get_trust_details(&self, party_id: Uuid) -> Result<Option<TrustDetailsRow>, DatabaseError> {
        let details = sqlx::query_as!(
            TrustDetailsRow,
            r#"
            SELECT
                trust_id,
                party_id,
                trust_name,
                trust_identification,
                established_date,
                trust_type as "trust_type: TrustType",
                is_revocable,
                governing_jurisdiction,
                created_at,
                updated_at
            FROM party_trust_details
            WHERE party_id = $1 AND upper(sys_period) IS NULL
            "#,
            party_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(details)
    }

    /// Gets partnership details for a party
    async fn get_partnership_details(&self, party_id: Uuid) -> Result<Option<PartnershipDetailsRow>, DatabaseError> {
        let details = sqlx::query_as!(
            PartnershipDetailsRow,
            r#"
            SELECT
                partnership_id,
                party_id,
                partnership_name,
                registration_number,
                tax_id,
                partnership_type as "partnership_type: PartnershipType",
                formation_date,
                formation_jurisdiction,
                created_at,
                updated_at
            FROM party_partnership_details
            WHERE party_id = $1 AND upper(sys_period) IS NULL
            "#,
            party_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(details)
    }

    /// Updates a member's ownership percentage
    ///
    /// # Arguments
    ///
    /// * `member_id` - The member record ID
    /// * `new_percentage` - The new ownership percentage
    ///
    /// # Returns
    ///
    /// The updated member record
    pub async fn update_member_ownership(
        &self,
        member_id: Uuid,
        new_percentage: Decimal,
    ) -> Result<PartyMemberRow, DatabaseError> {
        let now = Utc::now();

        let row = sqlx::query_as!(
            PartyMemberRow,
            r#"
            UPDATE party_members
            SET ownership_percentage = $2, updated_at = $3
            WHERE member_id = $1 AND upper(sys_period) IS NULL
            RETURNING
                member_id,
                party_id,
                member_party_id,
                role as "role: MemberRole",
                ownership_percentage,
                is_primary_contact,
                effective_from,
                effective_to,
                created_at,
                updated_at
            "#,
            member_id,
            new_percentage,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Sets the primary contact for a composite party
    ///
    /// Clears any existing primary contact and sets the new one.
    ///
    /// # Arguments
    ///
    /// * `party_id` - The composite party ID
    /// * `member_party_id` - The member party ID to set as primary contact
    pub async fn set_primary_contact(
        &self,
        party_id: Uuid,
        member_party_id: Uuid,
    ) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();

        // Clear existing primary contact
        sqlx::query!(
            r#"
            UPDATE party_members
            SET is_primary_contact = FALSE, updated_at = $2
            WHERE party_id = $1
              AND is_primary_contact = TRUE
              AND effective_to IS NULL
              AND upper(sys_period) IS NULL
            "#,
            party_id,
            now
        )
        .execute(&mut *tx)
        .await?;

        // Set new primary contact
        sqlx::query!(
            r#"
            UPDATE party_members
            SET is_primary_contact = TRUE, updated_at = $3
            WHERE party_id = $1
              AND member_party_id = $2
              AND effective_to IS NULL
              AND upper(sys_period) IS NULL
            "#,
            party_id,
            member_party_id,
            now
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}

// ============================================================================
// Type definitions
// ============================================================================

/// Party type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "party_type", rename_all = "snake_case")]
pub enum PartyType {
    Individual,
    Corporate,
    Agent,
    Broker,
    Beneficiary,
    Joint,
    Trust,
    Partnership,
}

/// Party composition enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "party_composition", rename_all = "snake_case")]
pub enum PartyComposition {
    Individual,
    Corporate,
    Joint,
    Trust,
    Partnership,
}

/// KYC (Know Your Customer) status
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "kyc_status", rename_all = "snake_case")]
pub enum KycStatus {
    Pending,
    InProgress,
    Verified,
    Failed,
    Expired,
}

/// Member role enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "member_role", rename_all = "snake_case")]
pub enum MemberRole {
    PrimaryOwner,
    CoOwner,
    Trustee,
    TrustBeneficiary,
    Settlor,
    ManagingPartner,
    Partner,
    SilentPartner,
    AuthorizedSignatory,
    Director,
}

/// Joint type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "joint_type", rename_all = "snake_case")]
pub enum JointType {
    JointTenants,
    TenantsInCommon,
    CommunityProperty,
    Other,
}

/// Trust type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "trust_type", rename_all = "snake_case")]
pub enum TrustType {
    RevocableLiving,
    Ilit,
    CharitableRemainder,
    SpecialNeeds,
    Testamentary,
    Other,
}

/// Partnership type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "partnership_type", rename_all = "snake_case")]
pub enum PartnershipType {
    GeneralPartnership,
    LimitedPartnership,
    Llp,
    Other,
}

// ============================================================================
// Row types
// ============================================================================

/// Database row representation of a party version
#[derive(Debug, Clone)]
pub struct PartyRow {
    pub version_id: Uuid,
    pub party_id: Uuid,
    pub party_type: PartyType,
    pub composition: PartyComposition,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub company_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub date_of_birth: Option<chrono::NaiveDate>,
    pub tax_id: Option<String>,
    pub kyc_status: KycStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database row for party members
#[derive(Debug, Clone)]
pub struct PartyMemberRow {
    pub member_id: Uuid,
    pub party_id: Uuid,
    pub member_party_id: Uuid,
    pub role: MemberRole,
    pub ownership_percentage: Option<Decimal>,
    pub is_primary_contact: bool,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database row for joint details
#[derive(Debug, Clone)]
pub struct JointDetailsRow {
    pub joint_id: Uuid,
    pub party_id: Uuid,
    pub display_name: String,
    pub joint_type: JointType,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database row for trust details
#[derive(Debug, Clone)]
pub struct TrustDetailsRow {
    pub trust_id: Uuid,
    pub party_id: Uuid,
    pub trust_name: String,
    pub trust_identification: Option<String>,
    pub established_date: Option<chrono::NaiveDate>,
    pub trust_type: TrustType,
    pub is_revocable: bool,
    pub governing_jurisdiction: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database row for partnership details
#[derive(Debug, Clone)]
pub struct PartnershipDetailsRow {
    pub partnership_id: Uuid,
    pub party_id: Uuid,
    pub partnership_name: String,
    pub registration_number: Option<String>,
    pub tax_id: Option<String>,
    pub partnership_type: PartnershipType,
    pub formation_date: Option<chrono::NaiveDate>,
    pub formation_jurisdiction: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Complete party with all composition details
#[derive(Debug, Clone)]
pub struct PartyWithComposition {
    pub party: PartyRow,
    pub joint_details: Option<JointDetailsRow>,
    pub trust_details: Option<TrustDetailsRow>,
    pub partnership_details: Option<PartnershipDetailsRow>,
    pub members: Vec<PartyMemberRow>,
}

// ============================================================================
// Input types
// ============================================================================

/// Data for creating a new party
#[derive(Debug, Clone)]
pub struct NewParty {
    pub party_type: PartyType,
    pub composition: PartyComposition,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub company_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub date_of_birth: Option<chrono::NaiveDate>,
    pub tax_id: Option<String>,
    pub kyc_status: KycStatus,
}

/// Data for creating a new party member
#[derive(Debug, Clone)]
pub struct NewPartyMember {
    pub member_party_id: Uuid,
    pub role: MemberRole,
    pub ownership_percentage: Option<Decimal>,
    pub is_primary_contact: bool,
}

/// Data for creating joint details
#[derive(Debug, Clone)]
pub struct NewJointDetails {
    pub display_name: String,
    pub joint_type: JointType,
    pub notes: Option<String>,
}

/// Data for creating trust details
#[derive(Debug, Clone)]
pub struct NewTrustDetails {
    pub trust_name: String,
    pub trust_id: Option<String>,
    pub established_date: Option<chrono::NaiveDate>,
    pub trust_type: TrustType,
    pub is_revocable: bool,
    pub governing_jurisdiction: Option<String>,
}

/// Data for creating partnership details
#[derive(Debug, Clone)]
pub struct NewPartnershipDetails {
    pub partnership_name: String,
    pub registration_number: Option<String>,
    pub tax_id: Option<String>,
    pub partnership_type: PartnershipType,
    pub formation_date: Option<chrono::NaiveDate>,
    pub formation_jurisdiction: Option<String>,
}
