-- A model-independent billing/authentication witness can authorize a bounded probe,
-- never a caller inference/evaluation request or a fabricated model qualification.
CREATE TABLE verification_account_authorities (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    connection_id uuid NOT NULL,
    generation bigint NOT NULL,
    operation_id uuid NOT NULL,
    adapter_revision text NOT NULL CHECK(length(adapter_revision) BETWEEN 1 AND 120),
    endpoint_policy_hash text NOT NULL CHECK(length(endpoint_policy_hash)=64),
    currency varchar(3) NOT NULL CHECK(currency ~ '^[A-Z]{3}$'),
    billing_tier text NOT NULL CHECK(length(billing_tier) BETWEEN 1 AND 120),
    statement_origin text NOT NULL CHECK(length(statement_origin) BETWEEN 1 AND 240),
    statement_digest text NOT NULL CHECK(length(statement_digest)=64),
    statement_usage numeric(38,18) NOT NULL CHECK(statement_usage>=0),
    billing_capabilities jsonb NOT NULL CHECK(jsonb_typeof(billing_capabilities)='object'),
    observed_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL,
    state text NOT NULL CHECK(state IN ('active','invalidated')),
    CHECK(expires_at>observed_at AND expires_at<=observed_at+interval '1 hour'),
    FOREIGN KEY(installation_id,connection_id) REFERENCES connections(installation_id,id),
    FOREIGN KEY(connection_id,generation) REFERENCES connection_generations(connection_id,generation),
    FOREIGN KEY(installation_id,operation_id) REFERENCES operations(installation_id,id),
    UNIQUE(installation_id,id),UNIQUE(installation_id,connection_id,generation,id)
);
CREATE TRIGGER verification_account_binding_immutable BEFORE UPDATE OF id,installation_id,connection_id,generation,operation_id,adapter_revision,endpoint_policy_hash,currency,billing_tier,statement_origin,statement_digest,statement_usage,billing_capabilities,observed_at,expires_at OR DELETE ON verification_account_authorities
FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
ALTER TABLE attempts ALTER COLUMN qualification_id DROP NOT NULL;
ALTER TABLE attempts ADD COLUMN account_authority_id uuid;
ALTER TABLE attempts ADD CONSTRAINT attempt_account_authority_fk FOREIGN KEY(installation_id,connection_id,generation,account_authority_id)
REFERENCES verification_account_authorities(installation_id,connection_id,generation,id);
ALTER TABLE attempts ADD CONSTRAINT attempt_one_target_authority CHECK(num_nonnulls(qualification_id,account_authority_id)=1);
CREATE TRIGGER attempt_account_authority_immutable BEFORE UPDATE OF account_authority_id ON attempts
FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE FUNCTION enforce_verification_account_purpose() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.account_authority_id IS NOT NULL AND NOT EXISTS(
        SELECT 1 FROM requests r WHERE r.installation_id=NEW.installation_id AND r.id=NEW.request_id AND r.request_kind='verification'
    ) THEN RAISE EXCEPTION 'account authority only permits verification' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER attempt_verification_account_purpose BEFORE INSERT ON attempts
FOR EACH ROW EXECUTE FUNCTION enforce_verification_account_purpose();
