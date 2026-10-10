CREATE TABLE endpoint_policies (
    installation_id uuid NOT NULL REFERENCES installations(id),
    policy_ref text NOT NULL CHECK(length(policy_ref) BETWEEN 1 AND 120),
    provider_kind text NOT NULL CHECK(provider_kind IN ('openai_compatible','ollama','zai','chatgpt_managed')),
    policy jsonb NOT NULL CHECK(jsonb_typeof(policy)='object'),
    policy_hash text NOT NULL CHECK(length(policy_hash)=64),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(installation_id,policy_ref)
);
CREATE TRIGGER endpoint_policy_immutable BEFORE UPDATE OR DELETE ON endpoint_policies FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
ALTER TABLE connection_generations ADD COLUMN endpoint_snapshot jsonb CHECK(endpoint_snapshot IS NULL OR jsonb_typeof(endpoint_snapshot)='object');
CREATE TRIGGER connection_generation_snapshot_immutable BEFORE UPDATE OF connection_id,generation,adapter_revision,endpoint_policy_hash,endpoint_snapshot,created_at OR DELETE ON connection_generations FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
