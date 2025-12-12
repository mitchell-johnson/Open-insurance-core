-- Party Composition Migration
-- Adds support for composite parties (joint owners, trusts, partnerships)
-- This enables complex ownership structures while maintaining a single policyholder_id

-- Add new party types to the enum
ALTER TYPE party_type ADD VALUE IF NOT EXISTS 'joint';
ALTER TYPE party_type ADD VALUE IF NOT EXISTS 'trust';
ALTER TYPE party_type ADD VALUE IF NOT EXISTS 'partnership';

-- Create party composition enum
CREATE TYPE party_composition AS ENUM (
    'individual',
    'corporate',
    'joint',
    'trust',
    'partnership'
);

-- Create member role enum
CREATE TYPE member_role AS ENUM (
    'primary_owner',
    'co_owner',
    'trustee',
    'trust_beneficiary',
    'settlor',
    'managing_partner',
    'partner',
    'silent_partner',
    'authorized_signatory',
    'director'
);

-- Create joint type enum
CREATE TYPE joint_type AS ENUM (
    'joint_tenants',
    'tenants_in_common',
    'community_property',
    'other'
);

-- Create trust type enum
CREATE TYPE trust_type AS ENUM (
    'revocable_living',
    'ilit',
    'charitable_remainder',
    'special_needs',
    'testamentary',
    'other'
);

-- Create partnership type enum
CREATE TYPE partnership_type AS ENUM (
    'general_partnership',
    'limited_partnership',
    'llp',
    'other'
);

-- Add composition column to party_versions
ALTER TABLE party_versions
ADD COLUMN IF NOT EXISTS composition party_composition;

-- Set default composition based on existing party_type
UPDATE party_versions
SET composition = CASE
    WHEN party_type = 'individual' THEN 'individual'::party_composition
    WHEN party_type = 'corporate' THEN 'corporate'::party_composition
    ELSE 'individual'::party_composition
END
WHERE composition IS NULL;

-- Make composition NOT NULL after setting defaults
ALTER TABLE party_versions
ALTER COLUMN composition SET NOT NULL;

-- Create joint_details table for joint ownership arrangements
CREATE TABLE party_joint_details (
    joint_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    party_id UUID NOT NULL,
    display_name VARCHAR(200) NOT NULL,
    joint_type joint_type NOT NULL DEFAULT 'joint_tenants',
    notes TEXT,

    -- Bi-temporal columns
    valid_period tstzrange NOT NULL DEFAULT tstzrange(CURRENT_TIMESTAMP, NULL),
    sys_period tstzrange NOT NULL DEFAULT tstzrange(CURRENT_TIMESTAMP, NULL),

    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Prevent overlapping valid periods for the same party
    EXCLUDE USING gist (
        party_id WITH =,
        valid_period WITH &&
    ) WHERE (upper(sys_period) IS NULL)
);

CREATE INDEX idx_party_joint_details_party ON party_joint_details(party_id);
CREATE INDEX idx_party_joint_details_current ON party_joint_details(party_id)
    WHERE upper(sys_period) IS NULL;

-- Create trust_details table for trust entities
CREATE TABLE party_trust_details (
    trust_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    party_id UUID NOT NULL,
    trust_name VARCHAR(200) NOT NULL,
    trust_identification VARCHAR(100),
    established_date DATE,
    trust_type trust_type NOT NULL DEFAULT 'revocable_living',
    is_revocable BOOLEAN NOT NULL DEFAULT TRUE,
    governing_jurisdiction VARCHAR(100),

    -- Bi-temporal columns
    valid_period tstzrange NOT NULL DEFAULT tstzrange(CURRENT_TIMESTAMP, NULL),
    sys_period tstzrange NOT NULL DEFAULT tstzrange(CURRENT_TIMESTAMP, NULL),

    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    EXCLUDE USING gist (
        party_id WITH =,
        valid_period WITH &&
    ) WHERE (upper(sys_period) IS NULL)
);

CREATE INDEX idx_party_trust_details_party ON party_trust_details(party_id);
CREATE INDEX idx_party_trust_details_current ON party_trust_details(party_id)
    WHERE upper(sys_period) IS NULL;

-- Create partnership_details table for partnership entities
CREATE TABLE party_partnership_details (
    partnership_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    party_id UUID NOT NULL,
    partnership_name VARCHAR(200) NOT NULL,
    registration_number VARCHAR(100),
    tax_id VARCHAR(100),
    partnership_type partnership_type NOT NULL DEFAULT 'general_partnership',
    formation_date DATE,
    formation_jurisdiction VARCHAR(100),

    -- Bi-temporal columns
    valid_period tstzrange NOT NULL DEFAULT tstzrange(CURRENT_TIMESTAMP, NULL),
    sys_period tstzrange NOT NULL DEFAULT tstzrange(CURRENT_TIMESTAMP, NULL),

    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    EXCLUDE USING gist (
        party_id WITH =,
        valid_period WITH &&
    ) WHERE (upper(sys_period) IS NULL)
);

CREATE INDEX idx_party_partnership_details_party ON party_partnership_details(party_id);
CREATE INDEX idx_party_partnership_details_current ON party_partnership_details(party_id)
    WHERE upper(sys_period) IS NULL;

-- Create party_members table for composite party membership
CREATE TABLE party_members (
    member_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    party_id UUID NOT NULL,           -- The composite party (joint, trust, partnership)
    member_party_id UUID NOT NULL,    -- The member party (individual, corporate)
    role member_role NOT NULL,
    ownership_percentage NUMERIC(5, 2), -- 0.00 to 100.00
    is_primary_contact BOOLEAN NOT NULL DEFAULT FALSE,
    effective_from TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    effective_to TIMESTAMPTZ,         -- NULL if currently active

    -- Bi-temporal columns for audit trail
    sys_period tstzrange NOT NULL DEFAULT tstzrange(CURRENT_TIMESTAMP, NULL),

    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Constraints
    CONSTRAINT chk_ownership_percentage CHECK (
        ownership_percentage IS NULL
        OR (ownership_percentage >= 0 AND ownership_percentage <= 100)
    ),
    CONSTRAINT chk_effective_dates CHECK (
        effective_to IS NULL OR effective_to > effective_from
    ),
    -- Prevent a party from being a member of itself
    CONSTRAINT chk_no_self_membership CHECK (party_id != member_party_id)
);

-- Create indexes for party_members
CREATE INDEX idx_party_members_party ON party_members(party_id);
CREATE INDEX idx_party_members_member ON party_members(member_party_id);
CREATE INDEX idx_party_members_active ON party_members(party_id)
    WHERE effective_to IS NULL;
CREATE INDEX idx_party_members_role ON party_members(party_id, role);

-- Create unique constraint to prevent duplicate active memberships
CREATE UNIQUE INDEX idx_party_members_unique_active ON party_members(party_id, member_party_id)
    WHERE effective_to IS NULL AND upper(sys_period) IS NULL;

-- Create view for current party members
CREATE OR REPLACE VIEW party_members_current AS
SELECT
    pm.member_id,
    pm.party_id,
    pm.member_party_id,
    pm.role,
    pm.ownership_percentage,
    pm.is_primary_contact,
    pm.effective_from,
    pm.effective_to,
    pm.created_at,
    pm.updated_at
FROM party_members pm
WHERE pm.effective_to IS NULL
  AND upper(pm.sys_period) IS NULL;

-- Create view for party with composition details
CREATE OR REPLACE VIEW party_with_composition AS
SELECT
    pv.version_id,
    pv.party_id,
    pv.party_type,
    pv.composition,
    pv.first_name,
    pv.last_name,
    pv.company_name,
    pv.email,
    pv.phone,
    pv.date_of_birth,
    pv.tax_id,
    pv.kyc_status,
    pv.created_at,
    pv.updated_at,
    -- Joint details
    pjd.display_name AS joint_display_name,
    pjd.joint_type,
    pjd.notes AS joint_notes,
    -- Trust details
    ptd.trust_name,
    ptd.trust_identification,
    ptd.established_date AS trust_established_date,
    ptd.trust_type,
    ptd.is_revocable AS trust_is_revocable,
    ptd.governing_jurisdiction AS trust_jurisdiction,
    -- Partnership details
    ppd.partnership_name,
    ppd.registration_number AS partnership_registration,
    ppd.partnership_type,
    ppd.formation_date AS partnership_formation_date,
    ppd.formation_jurisdiction AS partnership_jurisdiction
FROM party_versions pv
LEFT JOIN party_joint_details pjd
    ON pv.party_id = pjd.party_id
    AND upper(pjd.sys_period) IS NULL
LEFT JOIN party_trust_details ptd
    ON pv.party_id = ptd.party_id
    AND upper(ptd.sys_period) IS NULL
LEFT JOIN party_partnership_details ppd
    ON pv.party_id = ppd.party_id
    AND upper(ppd.sys_period) IS NULL
WHERE upper(pv.sys_period) IS NULL;

-- Function to get all member party IDs for a composite party
CREATE OR REPLACE FUNCTION get_party_member_ids(p_party_id UUID)
RETURNS TABLE(member_party_id UUID) AS $$
BEGIN
    RETURN QUERY
    SELECT pm.member_party_id
    FROM party_members pm
    WHERE pm.party_id = p_party_id
      AND pm.effective_to IS NULL
      AND upper(pm.sys_period) IS NULL;
END;
$$ LANGUAGE plpgsql;

-- Function to check if a party can be a member of another party
-- (prevents circular references)
CREATE OR REPLACE FUNCTION check_party_membership(
    p_party_id UUID,
    p_member_party_id UUID
) RETURNS BOOLEAN AS $$
DECLARE
    v_composition party_composition;
    v_member_composition party_composition;
    v_has_circular BOOLEAN;
BEGIN
    -- Get compositions
    SELECT composition INTO v_composition
    FROM party_versions
    WHERE party_id = p_party_id AND upper(sys_period) IS NULL;

    SELECT composition INTO v_member_composition
    FROM party_versions
    WHERE party_id = p_member_party_id AND upper(sys_period) IS NULL;

    -- Only composite parties can have members
    IF v_composition NOT IN ('joint', 'trust', 'partnership') THEN
        RETURN FALSE;
    END IF;

    -- Members should typically be individuals or corporates
    -- (but we allow composite members for complex structures)

    -- Check for circular reference: member should not already contain party_id
    SELECT EXISTS(
        WITH RECURSIVE member_tree AS (
            SELECT pm.member_party_id, 1 AS depth
            FROM party_members pm
            WHERE pm.party_id = p_member_party_id
              AND pm.effective_to IS NULL
              AND upper(pm.sys_period) IS NULL
            UNION ALL
            SELECT pm.member_party_id, mt.depth + 1
            FROM party_members pm
            JOIN member_tree mt ON pm.party_id = mt.member_party_id
            WHERE pm.effective_to IS NULL
              AND upper(pm.sys_period) IS NULL
              AND mt.depth < 10  -- Prevent infinite recursion
        )
        SELECT 1 FROM member_tree WHERE member_party_id = p_party_id
    ) INTO v_has_circular;

    RETURN NOT v_has_circular;
END;
$$ LANGUAGE plpgsql;

-- Trigger to validate membership before insert/update
CREATE OR REPLACE FUNCTION validate_party_membership()
RETURNS TRIGGER AS $$
BEGIN
    IF NOT check_party_membership(NEW.party_id, NEW.member_party_id) THEN
        RAISE EXCEPTION 'Invalid party membership: circular reference or invalid party types';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_validate_party_membership
    BEFORE INSERT OR UPDATE ON party_members
    FOR EACH ROW
    EXECUTE FUNCTION validate_party_membership();

-- Add audit trigger for party_members
CREATE TRIGGER audit_party_members
    AFTER INSERT OR UPDATE OR DELETE ON party_members
    FOR EACH ROW
    EXECUTE FUNCTION audit_trigger();

-- Create the audit_trigger function if it doesn't exist
CREATE OR REPLACE FUNCTION audit_trigger()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        INSERT INTO audit_log (
            user_id, action, entity_type, entity_id, old_values, new_values
        ) VALUES (
            COALESCE(current_setting('app.user_id', true), 'system'),
            'DELETE',
            TG_TABLE_NAME,
            OLD.member_id,
            to_jsonb(OLD),
            NULL
        );
        RETURN OLD;
    ELSIF TG_OP = 'UPDATE' THEN
        INSERT INTO audit_log (
            user_id, action, entity_type, entity_id, old_values, new_values
        ) VALUES (
            COALESCE(current_setting('app.user_id', true), 'system'),
            'UPDATE',
            TG_TABLE_NAME,
            NEW.member_id,
            to_jsonb(OLD),
            to_jsonb(NEW)
        );
        RETURN NEW;
    ELSIF TG_OP = 'INSERT' THEN
        INSERT INTO audit_log (
            user_id, action, entity_type, entity_id, old_values, new_values
        ) VALUES (
            COALESCE(current_setting('app.user_id', true), 'system'),
            'INSERT',
            TG_TABLE_NAME,
            NEW.member_id,
            NULL,
            to_jsonb(NEW)
        );
        RETURN NEW;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Comments for documentation
COMMENT ON TABLE party_members IS 'Links member parties to composite parties (joint, trust, partnership)';
COMMENT ON COLUMN party_members.party_id IS 'The composite party that contains the members';
COMMENT ON COLUMN party_members.member_party_id IS 'The individual or corporate party that is a member';
COMMENT ON COLUMN party_members.role IS 'The role this member plays in the composite party';
COMMENT ON COLUMN party_members.ownership_percentage IS 'Percentage ownership (0-100) for ownership roles';
COMMENT ON COLUMN party_members.is_primary_contact IS 'Whether this member is the primary contact for the party';

COMMENT ON TABLE party_joint_details IS 'Additional details for joint ownership parties';
COMMENT ON TABLE party_trust_details IS 'Additional details for trust parties';
COMMENT ON TABLE party_partnership_details IS 'Additional details for partnership parties';
