CREATE TABLE virtual_models (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    slug varchar(63) NOT NULL CHECK(slug ~ '^[a-z][a-z0-9-]{1,62}$'),
    display_name varchar(120) NOT NULL CHECK(length(trim(display_name))>0),
    draft jsonb NOT NULL CHECK(jsonb_typeof(draft)='object'),
    draft_version bigint NOT NULL CHECK(draft_version>0),
    active_revision_id uuid,
    status text NOT NULL CHECK(status IN ('draft','active','disabled','archived')),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(installation_id,slug),UNIQUE(installation_id,id)
);
CREATE TRIGGER profile_identity_immutable BEFORE UPDATE OF id,installation_id,slug,created_at OR DELETE ON virtual_models
FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TABLE profile_draft_revisions (
    installation_id uuid NOT NULL,
    virtual_model_id uuid NOT NULL,
    version bigint NOT NULL CHECK(version>0),
    configuration jsonb NOT NULL CHECK(jsonb_typeof(configuration)='object'),
    config_hash text NOT NULL CHECK(length(config_hash)=64),
    operation_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(virtual_model_id,version),
    FOREIGN KEY(installation_id,virtual_model_id) REFERENCES virtual_models(installation_id,id),
    FOREIGN KEY(installation_id,operation_id) REFERENCES operations(installation_id,id),
    UNIQUE(installation_id,virtual_model_id,version)
);
CREATE TRIGGER profile_draft_revision_immutable BEFORE UPDATE OR DELETE ON profile_draft_revisions
FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TABLE profile_draft_targets (
    installation_id uuid NOT NULL,
    virtual_model_id uuid NOT NULL,
    version bigint NOT NULL,
    ordinal integer NOT NULL CHECK(ordinal BETWEEN 1 AND 5),
    connection_id uuid NOT NULL,
    generation bigint NOT NULL,
    model_id text NOT NULL CHECK(length(model_id) BETWEEN 1 AND 256),
    PRIMARY KEY(virtual_model_id,version,ordinal),
    FOREIGN KEY(installation_id,virtual_model_id,version) REFERENCES profile_draft_revisions(installation_id,virtual_model_id,version),
    FOREIGN KEY(installation_id,connection_id) REFERENCES connections(installation_id,id),
    FOREIGN KEY(connection_id,generation) REFERENCES connection_generations(connection_id,generation),
    UNIQUE(virtual_model_id,version,connection_id,generation,model_id)
);
CREATE TRIGGER profile_draft_target_immutable BEFORE UPDATE OR DELETE ON profile_draft_targets
FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
ALTER TABLE virtual_models ADD CONSTRAINT profile_current_draft_fk FOREIGN KEY(installation_id,id,draft_version)
REFERENCES profile_draft_revisions(installation_id,virtual_model_id,version) DEFERRABLE INITIALLY DEFERRED;
ALTER TABLE probe_snapshots ADD CONSTRAINT probe_own_draft_fk FOREIGN KEY(installation_id,draft_model_id,draft_version)
REFERENCES profile_draft_revisions(installation_id,virtual_model_id,version) NOT VALID;
-- Existing pinned snapshots are not rewritten. Nonempty validation belongs to S7.
