-- Current tier belongs to the qualified connection generation, not its display name.
ALTER TABLE connection_generations ADD COLUMN billing_tier text CHECK(length(billing_tier) BETWEEN 1 AND 120);
CREATE TABLE pricing_source_policies (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    connection_id uuid NOT NULL,
    model_id text NOT NULL CHECK(length(model_id) BETWEEN 1 AND 120),
    currency varchar(3) NOT NULL CHECK(currency ~ '^[A-Z]{3}$'),
    version bigint NOT NULL DEFAULT 0 CHECK(version>=0),
    FOREIGN KEY(installation_id,connection_id) REFERENCES connections(installation_id,id),
    UNIQUE(installation_id,connection_id,model_id,currency),UNIQUE(installation_id,id)
);
CREATE TRIGGER pricing_policy_identity_immutable BEFORE UPDATE OF id,installation_id,connection_id,model_id,currency OR DELETE ON pricing_source_policies FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TABLE pricing_source_revisions (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    policy_id uuid NOT NULL,
    mode text NOT NULL CHECK(mode IN ('manual','provider_auto')),
    manual_price_revision_id uuid,
    expected_version bigint NOT NULL CHECK(expected_version>=0),
    version bigint NOT NULL CHECK(version>0 AND version=expected_version+1),
    effective_from timestamptz NOT NULL,
    effective_to timestamptz,
    catalog_observed_at timestamptz,
    actor_subject text NOT NULL CHECK(length(actor_subject) BETWEEN 1 AND 256),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK(effective_to IS NULL OR effective_to>effective_from),
    CHECK((mode='manual' AND manual_price_revision_id IS NOT NULL) OR (mode='provider_auto' AND manual_price_revision_id IS NULL)),
    FOREIGN KEY(installation_id,policy_id) REFERENCES pricing_source_policies(installation_id,id),
    FOREIGN KEY(installation_id,manual_price_revision_id) REFERENCES price_revisions(installation_id,id),
    UNIQUE(installation_id,id),UNIQUE(policy_id,version),UNIQUE(policy_id,effective_from)
);
CREATE TRIGGER pricing_source_immutable BEFORE UPDATE OR DELETE ON pricing_source_revisions FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
-- Only the internal qualified catalog adapter writes this provenance. Human source labels are not authority.
CREATE TABLE catalog_price_origins (
    price_revision_id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    catalog_model_id uuid NOT NULL REFERENCES upstream_models(id),
    observed_at timestamptz NOT NULL,
    FOREIGN KEY(installation_id,price_revision_id) REFERENCES price_revisions(installation_id,id)
);
CREATE TRIGGER catalog_origin_immutable BEFORE UPDATE OR DELETE ON catalog_price_origins FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE INDEX pricing_source_timeline ON pricing_source_revisions(policy_id,effective_from DESC);
CREATE VIEW eligible_catalog_prices AS
SELECT p.id,p.installation_id,p.connection_id,p.model_id,p.currency,p.tier,p.effective_from,p.effective_to,o.observed_at
FROM catalog_price_origins o
JOIN price_revisions p ON p.installation_id=o.installation_id AND p.id=o.price_revision_id
JOIN upstream_models m ON m.id=o.catalog_model_id AND m.connection_id=p.connection_id AND m.provider_model_id=p.model_id AND m.observed_at=o.observed_at
JOIN connections c ON c.installation_id=p.installation_id AND c.id=p.connection_id AND c.generation=m.generation
JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation AND g.billing_tier=p.tier
WHERE c.status='enabled' AND g.authorization_state='active'
AND EXISTS(SELECT 1 FROM runtime_qualifications q JOIN verification_evidence e ON e.installation_id=q.installation_id AND e.id=q.proof_id
    WHERE q.installation_id=c.installation_id AND q.connection_id=c.id AND q.generation=c.generation AND q.provider_model_id=p.model_id
    AND q.billing_currency=p.currency AND q.state='active' AND q.adapter_revision=g.adapter_revision AND q.endpoint_policy_hash=g.endpoint_policy_hash
    AND q.capabilities @> '{"pricing_reader":true}' AND e.state='verified' AND e.expires_at>clock_timestamp());
