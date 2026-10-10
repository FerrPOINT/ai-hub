CREATE TABLE catalog_snapshots (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    connection_id uuid NOT NULL,
    generation bigint NOT NULL,
    source_digest text NOT NULL CHECK(length(source_digest)=64),
    models jsonb NOT NULL CHECK(jsonb_typeof(models)='array' AND jsonb_array_length(models)<=1000),
    observed_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY(installation_id,connection_id) REFERENCES connections(installation_id,id),
    FOREIGN KEY(connection_id,generation) REFERENCES connection_generations(connection_id,generation),
    UNIQUE(installation_id,connection_id,generation,id)
);
CREATE TRIGGER catalog_snapshot_immutable BEFORE UPDATE OR DELETE ON catalog_snapshots FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE UNIQUE INDEX catalog_snapshot_generation ON catalog_snapshots(connection_id,generation,id);
ALTER TABLE connection_generations ADD COLUMN catalog_snapshot_id uuid;
ALTER TABLE connection_generations ADD CONSTRAINT current_catalog_snapshot FOREIGN KEY(connection_id,generation,catalog_snapshot_id) REFERENCES catalog_snapshots(connection_id,generation,id);
