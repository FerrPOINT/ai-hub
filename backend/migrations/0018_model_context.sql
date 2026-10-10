CREATE TABLE model_context_revisions (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL,
    connection_id uuid NOT NULL,
    model_id text NOT NULL CHECK(length(model_id) BETWEEN 1 AND 256),
    version bigint NOT NULL CHECK(version>0),
    context_window_tokens bigint NOT NULL CHECK(context_window_tokens BETWEEN 1 AND 4294967295),
    operation_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    FOREIGN KEY(installation_id,connection_id) REFERENCES connections(installation_id,id),
    FOREIGN KEY(installation_id,operation_id) REFERENCES operations(installation_id,id),
    UNIQUE(installation_id,connection_id,model_id,version),
    UNIQUE(installation_id,connection_id,model_id,version,id)
);
CREATE TRIGGER model_context_revision_immutable BEFORE UPDATE OR DELETE ON model_context_revisions
FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TABLE model_context_preferences (
    installation_id uuid NOT NULL,
    connection_id uuid NOT NULL,
    model_id text NOT NULL,
    version bigint NOT NULL CHECK(version>0),
    current_revision_id uuid NOT NULL,
    PRIMARY KEY(connection_id,model_id),
    FOREIGN KEY(installation_id,connection_id,model_id,version,current_revision_id)
      REFERENCES model_context_revisions(installation_id,connection_id,model_id,version,id)
);
CREATE TRIGGER model_context_identity_immutable BEFORE UPDATE OF installation_id,connection_id,model_id OR DELETE ON model_context_preferences
FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
ALTER TABLE runtime_qualifications ADD COLUMN model_context_version bigint CHECK(model_context_version>=0);
ALTER TABLE runtime_qualifications ADD COLUMN model_context_revision_id uuid;
-- NULL is legacy unqualified; 0 explicitly witnesses absence at proof creation.
ALTER TABLE runtime_qualifications ADD CONSTRAINT qualified_model_context_shape CHECK(
    CASE WHEN model_context_version IS NULL THEN model_context_revision_id IS NULL
         WHEN model_context_version=0 THEN model_context_revision_id IS NULL
         ELSE model_context_revision_id IS NOT NULL END);
ALTER TABLE runtime_qualifications ADD CONSTRAINT qualified_model_context_fk
FOREIGN KEY(installation_id,connection_id,provider_model_id,model_context_version,model_context_revision_id)
REFERENCES model_context_revisions(installation_id,connection_id,model_id,version,id);
CREATE TRIGGER qualification_context_immutable BEFORE UPDATE OF model_context_version,model_context_revision_id ON runtime_qualifications
FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
