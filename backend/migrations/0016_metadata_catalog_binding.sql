-- The immutable catalog receipt must belong to the refresh's exact owner/generation.
CREATE FUNCTION check_metadata_catalog_binding() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM metadata_refreshes m JOIN catalog_snapshots c
          ON c.installation_id=m.installation_id AND c.connection_id=m.connection_id
         AND c.generation=m.generation
        WHERE m.operation_id=NEW.operation_id AND c.id=NEW.catalog_snapshot_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='Metadata catalog ownership mismatch';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER metadata_catalog_binding BEFORE INSERT ON metadata_account_observations
FOR EACH ROW EXECUTE FUNCTION check_metadata_catalog_binding();
-- Validate existing facts without rewriting immutable observations.
DO $$ BEGIN
    IF EXISTS (
        SELECT 1 FROM metadata_account_observations a
        JOIN metadata_refreshes m ON m.operation_id=a.operation_id
        JOIN catalog_snapshots c ON c.id=a.catalog_snapshot_id
        WHERE c.installation_id<>m.installation_id OR c.connection_id<>m.connection_id
           OR c.generation<>m.generation
    ) THEN
        RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='Metadata catalog ownership mismatch';
    END IF;
END $$;
