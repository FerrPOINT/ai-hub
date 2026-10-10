-- Unit-qualified account statements permit financial catalog quotes before model proof.
-- This view does not authorize inference; admission checks the exact authority separately.
CREATE OR REPLACE VIEW eligible_catalog_prices AS
SELECT p.id,p.installation_id,p.connection_id,p.model_id,p.currency,p.tier,p.effective_from,p.effective_to,o.observed_at
FROM catalog_price_origins o
JOIN price_revisions p ON p.installation_id=o.installation_id AND p.id=o.price_revision_id
JOIN upstream_models m ON m.id=o.catalog_model_id AND m.connection_id=p.connection_id AND m.provider_model_id=p.model_id AND m.observed_at=o.observed_at
JOIN connections c ON c.installation_id=p.installation_id AND c.id=p.connection_id AND c.generation=m.generation
JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation AND g.billing_tier=p.tier
WHERE (
 (c.status='enabled' AND g.authorization_state='active' AND EXISTS(
    SELECT 1 FROM runtime_qualifications q JOIN verification_evidence e ON e.installation_id=q.installation_id AND e.id=q.proof_id
    WHERE q.installation_id=c.installation_id AND q.connection_id=c.id AND q.generation=c.generation AND q.provider_model_id=p.model_id
    AND q.billing_currency=p.currency AND q.state='active' AND q.adapter_revision=g.adapter_revision AND q.endpoint_policy_hash=g.endpoint_policy_hash
    AND q.capabilities @> '{"pricing_reader":true}' AND e.state='verified' AND e.expires_at>clock_timestamp()
 ))
 OR (c.status IN ('enabled','authorization_unknown') AND g.authorization_state IN ('prepared','active') AND EXISTS(
    SELECT 1 FROM verification_account_authorities a JOIN credential_versions v ON v.connection_id=a.connection_id AND v.generation=a.generation
    WHERE a.installation_id=c.installation_id AND a.connection_id=c.id AND a.generation=c.generation AND a.currency=p.currency
    AND a.billing_tier=p.tier AND a.state='active' AND a.observed_at<=clock_timestamp() AND a.expires_at>clock_timestamp()
    AND a.adapter_revision=g.adapter_revision AND a.endpoint_policy_hash=g.endpoint_policy_hash AND v.state IN ('prepared','active')
    AND a.billing_capabilities @> '{"pricing_reader":true}'
 ))
);
