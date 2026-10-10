CREATE TABLE metadata_refreshes (
    operation_id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    connection_id uuid NOT NULL,
    generation bigint NOT NULL,
    endpoint_policy_hash text NOT NULL CHECK(length(endpoint_policy_hash)=64),
    fence uuid NOT NULL CHECK(fence <> '00000000-0000-0000-0000-000000000000'),
    state text NOT NULL CHECK(state IN ('running','succeeded','failed','unknown')),
    lease_until timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY(installation_id,operation_id) REFERENCES operations(installation_id,id),
    FOREIGN KEY(installation_id,connection_id) REFERENCES connections(installation_id,id),
    FOREIGN KEY(connection_id,generation) REFERENCES connection_generations(connection_id,generation)
);
CREATE TRIGGER metadata_claim_immutable BEFORE UPDATE OF operation_id,installation_id,connection_id,generation,endpoint_policy_hash,fence,lease_until,created_at OR DELETE ON metadata_refreshes FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE INDEX metadata_refresh_expiry ON metadata_refreshes(installation_id,lease_until) WHERE state='running';
-- Sanitized observations are not cash expenses, a currency witness, or inference proof.
CREATE TABLE metadata_account_observations (
    operation_id uuid PRIMARY KEY REFERENCES metadata_refreshes(operation_id),
    catalog_snapshot_id uuid NOT NULL REFERENCES catalog_snapshots(id),
    source_digest text NOT NULL CHECK(length(source_digest)=64),
    summary jsonb NOT NULL CHECK(jsonb_typeof(summary)='object'
        AND summary - ARRAY['limit','remaining','usage','byok_usage','is_free_tier','is_management_key','include_byok_in_limit','expires_at','currency_status'] = '{}'::jsonb
        AND summary->>'currency_status'='unqualified'),
    observed_at timestamptz NOT NULL
);
CREATE TRIGGER metadata_account_immutable BEFORE UPDATE OR DELETE ON metadata_account_observations FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
