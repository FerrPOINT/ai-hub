-- S2a owner tables precede any provider proof I/O. No secret/rate/grant defaults.
CREATE TABLE connections (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    provider_id uuid NOT NULL,
    display_name varchar(120) NOT NULL CHECK(length(display_name)>0),
    endpoint_policy_ref text NOT NULL CHECK(length(endpoint_policy_ref) BETWEEN 1 AND 120),
    billing_mode text NOT NULL CHECK(billing_mode IN ('metered','subscription','local','unknown')),
    generation bigint NOT NULL DEFAULT 1 CHECK(generation>0),
    status text NOT NULL CHECK(status IN ('enabled','disabled','revoked','authorization_unknown')),
    version bigint NOT NULL DEFAULT 1 CHECK(version>0),
    FOREIGN KEY(installation_id,provider_id) REFERENCES providers(installation_id,id),
    UNIQUE(installation_id,id)
);
CREATE TABLE connection_generations (
    connection_id uuid NOT NULL REFERENCES connections(id),
    generation bigint NOT NULL CHECK(generation>0),
    authorization_state text NOT NULL CHECK(authorization_state IN ('absent','prepared','active','revoked','unknown')),
    adapter_revision text NOT NULL,
    endpoint_policy_hash text NOT NULL CHECK(length(endpoint_policy_hash)=64),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(connection_id,generation)
);
CREATE TABLE credential_versions (
    connection_id uuid NOT NULL,
    generation bigint NOT NULL,
    ciphertext bytea NOT NULL CHECK(octet_length(ciphertext)>=16),
    nonce bytea NOT NULL CHECK(octet_length(nonce)=12),
    key_id text NOT NULL,
    state text NOT NULL CHECK(state IN ('prepared','active','revoked')),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(connection_id,generation),
    FOREIGN KEY(connection_id,generation) REFERENCES connection_generations(connection_id,generation),
    UNIQUE(key_id,nonce)
);
CREATE TABLE upstream_models (
    id uuid PRIMARY KEY,
    connection_id uuid NOT NULL,
    generation bigint NOT NULL,
    provider_model_id text NOT NULL CHECK(length(provider_model_id) BETWEEN 1 AND 256),
    input_limit bigint CHECK(input_limit>0),
    output_limit bigint CHECK(output_limit>0),
    metadata jsonb NOT NULL CHECK(jsonb_typeof(metadata)='object'),
    observed_at timestamptz NOT NULL,
    FOREIGN KEY(connection_id,generation) REFERENCES connection_generations(connection_id,generation),
    UNIQUE(connection_id,generation,provider_model_id)
);
CREATE TABLE price_revisions (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    connection_id uuid NOT NULL,
    model_id text NOT NULL CHECK(length(model_id) BETWEEN 1 AND 256),
    tier text NOT NULL CHECK(length(tier) BETWEEN 1 AND 120),
    currency varchar(3) NOT NULL CHECK(currency ~ '^[A-Z]{3}$'),
    input_uncached numeric(30,12) NOT NULL CHECK(input_uncached>=0),
    input_cached numeric(30,12) CHECK(input_cached>=0),
    cache_write numeric(30,12) CHECK(cache_write>=0),
    output_billable numeric(30,12) NOT NULL CHECK(output_billable>=0),
    explicit_other numeric(30,12) CHECK(explicit_other>=0),
    request_fee numeric(38,18) CHECK(request_fee>=0),
    effective_from timestamptz NOT NULL,
    effective_to timestamptz,
    source text NOT NULL CHECK(length(source) BETWEEN 1 AND 256),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK(effective_to IS NULL OR effective_to>effective_from),
    FOREIGN KEY(installation_id,connection_id) REFERENCES connections(installation_id,id),
    UNIQUE(installation_id,id)
);
CREATE TRIGGER price_immutable BEFORE UPDATE OR DELETE ON price_revisions FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TABLE probe_snapshots (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    operation_id uuid NOT NULL,
    scope text NOT NULL CHECK(scope IN ('connection_model','profile_draft')),
    config_hash text NOT NULL CHECK(length(config_hash)=64),
    draft_model_id uuid,
    draft_version bigint CHECK(draft_version>0),
    configuration jsonb NOT NULL CHECK(jsonb_typeof(configuration)='object'),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK((scope='connection_model' AND draft_model_id IS NULL AND draft_version IS NULL) OR (scope='profile_draft' AND draft_model_id IS NOT NULL AND draft_version IS NOT NULL)),
    FOREIGN KEY(installation_id,operation_id) REFERENCES operations(installation_id,id),
    UNIQUE(installation_id,id)
);
CREATE TRIGGER probe_immutable BEFORE UPDATE OR DELETE ON probe_snapshots FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TABLE requests (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    client_id uuid NOT NULL,
    principal_id text NOT NULL,
    project_binding text NOT NULL,
    request_kind text NOT NULL CHECK(request_kind IN ('inference','verification','evaluation')),
    profile_revision_id uuid,
    probe_snapshot_id uuid,
    idempotency_key uuid NOT NULL,
    payload_hmac bytea NOT NULL CHECK(octet_length(payload_hmac)=32),
    state text NOT NULL CHECK(state IN ('admitted','dispatching','streaming','completed','failed','cancelled','unknown')),
    cancel_requested boolean NOT NULL DEFAULT false,
    admitted_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz,
    version bigint NOT NULL DEFAULT 1 CHECK(version>0),
    namespace_binding_id uuid,
    project_tariff_snapshot jsonb,
    CHECK((request_kind='verification' AND probe_snapshot_id IS NOT NULL AND profile_revision_id IS NULL) OR (request_kind IN ('inference','evaluation') AND profile_revision_id IS NOT NULL AND probe_snapshot_id IS NULL)),
    FOREIGN KEY(installation_id,client_id) REFERENCES clients(installation_id,id),
    FOREIGN KEY(installation_id,probe_snapshot_id) REFERENCES probe_snapshots(installation_id,id),
    FOREIGN KEY(installation_id,namespace_binding_id) REFERENCES namespace_bindings(installation_id,id),
    UNIQUE(installation_id,id), UNIQUE(installation_id,client_id,idempotency_key)
    -- Exact profile revision FK is attached by S2b before public inference is exposed.
);
CREATE TRIGGER request_snapshot_immutable BEFORE UPDATE OF installation_id,client_id,principal_id,project_binding,request_kind,profile_revision_id,probe_snapshot_id,idempotency_key,payload_hmac,admitted_at,namespace_binding_id,project_tariff_snapshot OR DELETE ON requests FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TABLE replay_payloads (
    request_id uuid PRIMARY KEY REFERENCES requests(id),
    encrypted_response bytea NOT NULL CHECK(octet_length(encrypted_response)>=16),
    nonce bytea NOT NULL CHECK(octet_length(nonce)=12),
    key_id text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    protocol text NOT NULL CHECK(protocol IN ('chat_completions','responses')),
    CHECK(expires_at>created_at AND expires_at<=created_at+interval '24 hours'),
    UNIQUE(key_id,nonce)
);
CREATE TABLE attempts (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    request_id uuid NOT NULL,
    ordinal integer NOT NULL CHECK(ordinal BETWEEN 1 AND 5),
    connection_id uuid NOT NULL,
    generation bigint NOT NULL,
    deployment_snapshot jsonb NOT NULL CHECK(jsonb_typeof(deployment_snapshot)='object'),
    state text NOT NULL CHECK(state IN ('intended','dispatched','streaming','completed','failed','cancelled','unknown')),
    accepted text NOT NULL CHECK(accepted IN ('not_accepted','accepted','unknown')),
    dispatch_intent_at timestamptz NOT NULL DEFAULT now(),
    dispatched_at timestamptz,
    first_content_at timestamptz,
    finished_at timestamptz,
    provider_request_id text,
    actual_model text,
    actual_model_verified boolean NOT NULL DEFAULT false,
    version bigint NOT NULL DEFAULT 1 CHECK(version>0),
    FOREIGN KEY(installation_id,request_id) REFERENCES requests(installation_id,id),
    FOREIGN KEY(installation_id,connection_id) REFERENCES connections(installation_id,id),
    FOREIGN KEY(connection_id,generation) REFERENCES connection_generations(connection_id,generation),
    UNIQUE(request_id,ordinal), UNIQUE(installation_id,id)
);
CREATE TRIGGER attempt_snapshot_immutable BEFORE UPDATE OF installation_id,request_id,ordinal,connection_id,generation,deployment_snapshot,dispatch_intent_at OR DELETE ON attempts FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TABLE usage_facts (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    attempt_id uuid NOT NULL,
    source text NOT NULL CHECK(length(source) BETWEEN 1 AND 256),
    source_event_id text NOT NULL CHECK(length(source_event_id) BETWEEN 1 AND 256),
    categories jsonb NOT NULL CHECK(jsonb_typeof(categories)='object'),
    provenance jsonb NOT NULL CHECK(jsonb_typeof(provenance)='object'),
    observed_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY(installation_id,attempt_id) REFERENCES attempts(installation_id,id),
    UNIQUE(installation_id,source,source_event_id)
);
CREATE TRIGGER usage_immutable BEFORE UPDATE OR DELETE ON usage_facts FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TABLE ledger_entries (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    attempt_id uuid,
    charge_key text NOT NULL CHECK(length(charge_key) BETWEEN 1 AND 256),
    kind text NOT NULL CHECK(kind IN ('reserve','release','charge','correction','subscription_charge')),
    amount numeric(38,18),
    currency varchar(3) NOT NULL CHECK(currency ~ '^[A-Z]{3}$'),
    confidence text NOT NULL CHECK(confidence IN ('confirmed','estimated','unknown')),
    original_entry_id uuid,
    price_revision_id uuid,
    source text NOT NULL CHECK(length(source) BETWEEN 1 AND 256),
    source_event_id text NOT NULL CHECK(length(source_event_id) BETWEEN 1 AND 256),
    evidence jsonb NOT NULL CHECK(jsonb_typeof(evidence)='object'),
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK((confidence='unknown' AND amount IS NULL AND kind='charge') OR (confidence IN ('confirmed','estimated') AND amount IS NOT NULL)),
    CHECK(amount IS NULL OR amount>=0 OR kind='correction'),
    CHECK(kind<>'correction' OR (original_entry_id IS NOT NULL AND evidence<>'{}')),
    CHECK(kind NOT IN ('reserve','release','charge') OR attempt_id IS NOT NULL),
    FOREIGN KEY(installation_id,attempt_id) REFERENCES attempts(installation_id,id),
    FOREIGN KEY(installation_id,price_revision_id) REFERENCES price_revisions(installation_id,id),
    UNIQUE(installation_id,id), UNIQUE(installation_id,source,source_event_id),
    FOREIGN KEY(installation_id,original_entry_id) REFERENCES ledger_entries(installation_id,id)
);
CREATE TRIGGER ledger_immutable BEFORE UPDATE OR DELETE ON ledger_entries FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TABLE budget_policies (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    scope_type text NOT NULL CHECK(scope_type IN ('installation','project','client','profile')),
    scope_id text NOT NULL CHECK(length(scope_id) BETWEEN 1 AND 120),
    currency varchar(3) NOT NULL CHECK(currency ~ '^[A-Z]{3}$'),
    period text NOT NULL CHECK(period IN ('utc_day','utc_month')),
    hard_limit numeric(38,18) NOT NULL CHECK(hard_limit>=0),
    thresholds jsonb NOT NULL CHECK(jsonb_typeof(thresholds)='array'),
    version bigint NOT NULL DEFAULT 1 CHECK(version>0),
    status text NOT NULL CHECK(status IN ('active','disabled')),
    namespace_binding_id uuid,
    FOREIGN KEY(installation_id,namespace_binding_id) REFERENCES namespace_bindings(installation_id,id),
    CHECK((scope_type='project' AND namespace_binding_id IS NOT NULL) OR (scope_type<>'project' AND namespace_binding_id IS NULL)),
    UNIQUE(installation_id,scope_type,scope_id,currency,period), UNIQUE(installation_id,id)
);
CREATE TABLE budget_periods (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    policy_id uuid NOT NULL,
    period_start timestamptz NOT NULL,
    period_end timestamptz NOT NULL,
    charged numeric(38,18) NOT NULL DEFAULT 0 CHECK(charged>=0),
    reserved numeric(38,18) NOT NULL DEFAULT 0 CHECK(reserved>=0),
    version bigint NOT NULL DEFAULT 1 CHECK(version>0),
    CHECK(period_end>period_start),
    FOREIGN KEY(installation_id,policy_id) REFERENCES budget_policies(installation_id,id),
    UNIQUE(policy_id,period_start), UNIQUE(installation_id,id)
);
CREATE TABLE reservations (
    installation_id uuid NOT NULL REFERENCES installations(id),
    attempt_id uuid NOT NULL,
    budget_period_id uuid NOT NULL,
    amount numeric(38,18) NOT NULL CHECK(amount>=0),
    currency varchar(3) NOT NULL CHECK(currency ~ '^[A-Z]{3}$'),
    state text NOT NULL CHECK(state IN ('held','released','settled')),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(attempt_id,budget_period_id),
    FOREIGN KEY(installation_id,attempt_id) REFERENCES attempts(installation_id,id),
    FOREIGN KEY(installation_id,budget_period_id) REFERENCES budget_periods(installation_id,id)
);
CREATE FUNCTION validate_reservation_currency() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM budget_periods p JOIN budget_policies b ON b.installation_id=p.installation_id AND b.id=p.policy_id WHERE p.installation_id=NEW.installation_id AND p.id=NEW.budget_period_id AND b.currency=NEW.currency) THEN
        RAISE EXCEPTION 'reservation currency mismatch' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER reservation_currency BEFORE INSERT OR UPDATE ON reservations FOR EACH ROW EXECUTE FUNCTION validate_reservation_currency();
