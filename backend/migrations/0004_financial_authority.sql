-- Trusted billing evidence and bounded purpose authority. No identity registry.
CREATE TABLE verification_evidence (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    probe_snapshot_id uuid NOT NULL,
    state text NOT NULL CHECK(state IN ('pending','verified','failed','expired')),
    child_evidence jsonb NOT NULL CHECK(jsonb_typeof(child_evidence)='array'),
    receipt_digest text CHECK(receipt_digest IS NULL OR length(receipt_digest)=64),
    expires_at timestamptz NOT NULL,
    FOREIGN KEY(installation_id,probe_snapshot_id) REFERENCES probe_snapshots(installation_id,id),
    UNIQUE(installation_id,id)
);
CREATE TABLE runtime_qualifications (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    connection_id uuid NOT NULL,
    generation bigint NOT NULL,
    provider_model_id text NOT NULL CHECK(length(provider_model_id) BETWEEN 1 AND 256),
    adapter_revision text NOT NULL,
    endpoint_policy_hash text NOT NULL CHECK(length(endpoint_policy_hash)=64),
    proof_id uuid NOT NULL,
    capabilities jsonb NOT NULL CHECK(jsonb_typeof(capabilities)='object'),
    state text NOT NULL CHECK(state IN ('active','invalidated')),
    invalidated_reason text,
    billing_currency varchar(3) CHECK(billing_currency IS NULL OR billing_currency ~ '^[A-Z]{3}$'),
    billing_currency_origin text,
    CHECK((billing_currency IS NULL)=(billing_currency_origin IS NULL)),
    FOREIGN KEY(installation_id,connection_id) REFERENCES connections(installation_id,id),
    FOREIGN KEY(connection_id,generation) REFERENCES connection_generations(connection_id,generation),
    FOREIGN KEY(installation_id,proof_id) REFERENCES verification_evidence(installation_id,id),
    UNIQUE(installation_id,id)
);
ALTER TABLE grants ADD CONSTRAINT grant_installation_id UNIQUE(installation_id,id);
CREATE TRIGGER qualification_binding_immutable BEFORE UPDATE OF installation_id,connection_id,generation,provider_model_id,adapter_revision,endpoint_policy_hash,proof_id,capabilities,billing_currency,billing_currency_origin OR DELETE ON runtime_qualifications FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TRIGGER evidence_binding_immutable BEFORE UPDATE OF installation_id,probe_snapshot_id,child_evidence,receipt_digest,expires_at OR DELETE ON verification_evidence FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
-- Earlier prototype schema has no product admissions; invented backfill authority is forbidden.
ALTER TABLE requests ADD COLUMN grant_id uuid NOT NULL;
ALTER TABLE requests ADD COLUMN operation_id uuid NOT NULL;
ALTER TABLE requests ADD CONSTRAINT request_operation_fk FOREIGN KEY(installation_id,operation_id) REFERENCES operations(installation_id,id);
CREATE TRIGGER request_operation_immutable BEFORE UPDATE OF operation_id ON requests FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
ALTER TABLE requests ADD CONSTRAINT request_grant_fk FOREIGN KEY(installation_id,grant_id) REFERENCES grants(installation_id,id);
CREATE TRIGGER request_grant_immutable BEFORE UPDATE OF grant_id ON requests FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE INDEX requests_grant_created ON requests(grant_id,admitted_at);
CREATE INDEX requests_client_active ON requests(client_id,state);
CREATE TABLE grant_accounts (
    installation_id uuid NOT NULL REFERENCES installations(id),
    grant_id uuid PRIMARY KEY,
    currency varchar(3) NOT NULL CHECK(currency ~ '^[A-Z]{3}$'),
    requests_count bigint NOT NULL DEFAULT 0 CHECK(requests_count>=0),
    charged numeric(38,18) NOT NULL DEFAULT 0 CHECK(charged>=0),
    reserved numeric(38,18) NOT NULL DEFAULT 0 CHECK(reserved>=0),
    version bigint NOT NULL DEFAULT 1 CHECK(version>0),
    FOREIGN KEY(installation_id,grant_id) REFERENCES grants(installation_id,id)
);
ALTER TABLE attempts ADD COLUMN qualification_id uuid NOT NULL;
ALTER TABLE attempts ADD COLUMN upper_provider_cost numeric(38,18) CHECK(upper_provider_cost>=0);
ALTER TABLE attempts ADD COLUMN currency varchar(3) NOT NULL CHECK(currency ~ '^[A-Z]{3}$');
ALTER TABLE attempts ADD CONSTRAINT attempt_qualification_fk FOREIGN KEY(installation_id,qualification_id) REFERENCES runtime_qualifications(installation_id,id);
CREATE TRIGGER attempt_authority_immutable BEFORE UPDATE OF qualification_id,upper_provider_cost,currency ON attempts FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
-- Display metadata preserves actual owner names; UUID identity and grants do not change.
ALTER TABLE namespace_bindings ALTER COLUMN label TYPE varchar(200);
