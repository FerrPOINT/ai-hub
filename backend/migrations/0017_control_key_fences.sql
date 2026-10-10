-- Keep the no-send witness independently of operation replay expiry.
CREATE UNIQUE INDEX operation_actor_key_identity ON operations(installation_id,principal_kind,principal_id,idempotency_key,id);
CREATE TABLE control_key_fences (
    installation_id uuid NOT NULL,
    principal_kind text NOT NULL CHECK(principal_kind='human'),
    principal_id text NOT NULL CHECK(length(principal_id)>0),
    idempotency_key uuid NOT NULL CHECK(idempotency_key<>'00000000-0000-0000-0000-000000000000'),
    operation_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(installation_id,principal_kind,principal_id,idempotency_key),
    FOREIGN KEY(installation_id,principal_kind,principal_id,idempotency_key,operation_id)
      REFERENCES operations(installation_id,principal_kind,principal_id,idempotency_key,id)
);
CREATE TRIGGER control_key_fence_immutable BEFORE UPDATE OR DELETE ON control_key_fences
FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
