-- DB-01. No cross-product FK or inferred authority from labels.
CREATE TABLE installations (
    id uuid PRIMARY KEY CHECK (id <> '00000000-0000-0000-0000-000000000000'),
    stable_key text NOT NULL UNIQUE CHECK (length(stable_key) BETWEEN 1 AND 120),
    status text NOT NULL CHECK (status IN ('active','blocked')),
    blocked_reason text,
    vault_key_fingerprint bytea NOT NULL CHECK (octet_length(vault_key_fingerprint)=32),
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE providers (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    kind text NOT NULL CHECK (kind IN ('openai_compatible','ollama','zai','chatgpt_managed')),
    display_name varchar(120) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (installation_id, kind), UNIQUE (installation_id,id)
);
CREATE TABLE operations (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    principal_kind text NOT NULL CHECK (principal_kind IN ('human','client','internal')),
    principal_id text NOT NULL CHECK (length(principal_id)>0),
    idempotency_key uuid NOT NULL,
    binding_hmac bytea NOT NULL CHECK (octet_length(binding_hmac)=32),
    action text NOT NULL,
    resource_id uuid,
    state text NOT NULL CHECK (state IN ('pending','succeeded','failed','unknown','cancelled')),
    safe_result jsonb,
    version bigint NOT NULL DEFAULT 1 CHECK (version>0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    UNIQUE (installation_id,principal_kind,principal_id,idempotency_key),
    UNIQUE (installation_id,id)
);
CREATE TABLE namespace_bindings (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    registry_instance_id uuid NOT NULL CHECK (registry_instance_id <> '00000000-0000-0000-0000-000000000000'),
    namespace_id uuid NOT NULL CHECK (namespace_id <> '00000000-0000-0000-0000-000000000000'),
    tracker_instance_id uuid NOT NULL CHECK (tracker_instance_id <> '00000000-0000-0000-0000-000000000000'),
    tracker_project_id uuid NOT NULL CHECK (tracker_project_id <> '00000000-0000-0000-0000-000000000000'),
    state text NOT NULL CHECK (state IN ('active','archived','unavailable')),
    generation bigint NOT NULL CHECK (generation>0),
    observed_at timestamptz NOT NULL,
    label varchar(120) NOT NULL CHECK (length(label)>0),
    tracker_project_key varchar(120) NOT NULL,
    UNIQUE (installation_id,registry_instance_id,namespace_id),
    UNIQUE (installation_id,id)
);
CREATE TABLE clients (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    application_key varchar(120) NOT NULL,
    project_binding text NOT NULL,
    allowed_profiles jsonb NOT NULL CHECK (jsonb_typeof(allowed_profiles)='array' AND jsonb_array_length(allowed_profiles)<=100),
    scopes jsonb NOT NULL CHECK (jsonb_typeof(scopes)='array' AND scopes <@ '["infer","read_own_usage","read_result"]'),
    cost_policy text NOT NULL CHECK (cost_policy IN ('budget_guaranteed','cost_unknown_allowed')),
    expires_at timestamptz NOT NULL,
    max_concurrency integer NOT NULL CHECK (max_concurrency BETWEEN 1 AND 100),
    max_rpm integer NOT NULL CHECK (max_rpm BETWEEN 1 AND 10000),
    status text NOT NULL CHECK (status IN ('enabled','disabled','revoked')),
    version bigint NOT NULL CHECK (version>0),
    namespace_binding_id uuid,
    FOREIGN KEY (installation_id,namespace_binding_id) REFERENCES namespace_bindings(installation_id,id),
    UNIQUE (installation_id,application_key), UNIQUE (installation_id,id)
);
CREATE TABLE client_keys (
    id uuid PRIMARY KEY,
    client_id uuid NOT NULL REFERENCES clients(id),
    key_hash bytea NOT NULL UNIQUE CHECK (octet_length(key_hash)=32),
    prefix text NOT NULL,
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz
);
CREATE TABLE grants (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    principal_id text NOT NULL,
    client_id uuid,
    project_binding text NOT NULL,
    action text NOT NULL CHECK (action IN ('metadata.read','audit.read','artifact.read','read_result','infer','verification','evaluation')),
    virtual_model_id uuid,
    bounds jsonb NOT NULL CHECK (jsonb_typeof(bounds)='object'),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    namespace_binding_id uuid,
    FOREIGN KEY (installation_id,namespace_binding_id) REFERENCES namespace_bindings(installation_id,id),
    FOREIGN KEY (installation_id,client_id) REFERENCES clients(installation_id,id)
    -- virtual_model FK is introduced by S2 after its owner table exists.
);
CREATE INDEX grants_subject_scope ON grants(installation_id,principal_id,action,project_binding);
CREATE TABLE audit_events (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    actor text NOT NULL,
    action text NOT NULL,
    object_id text NOT NULL,
    revision text,
    operation_id uuid NOT NULL,
    reason text,
    happened_at timestamptz NOT NULL DEFAULT now(),
    namespace_binding_id uuid,
    FOREIGN KEY (installation_id,operation_id) REFERENCES operations(installation_id,id),
    FOREIGN KEY (installation_id,namespace_binding_id) REFERENCES namespace_bindings(installation_id,id)
);
CREATE INDEX audit_scope_time ON audit_events(installation_id,happened_at DESC,id);
CREATE FUNCTION forbid_immutable_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'immutable fact cannot be updated or deleted' USING ERRCODE='23514';
END;
$$;
CREATE TRIGGER audit_immutable BEFORE UPDATE OR DELETE ON audit_events FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TRIGGER installation_immutable BEFORE UPDATE OF id,stable_key,vault_key_fingerprint,created_at OR DELETE ON installations FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
