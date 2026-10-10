-- A dispatch claim is never reclaimed for a second provider send.
ALTER TABLE attempts ADD COLUMN dispatch_owner_id uuid;
ALTER TABLE attempts ADD COLUMN dispatch_fence uuid;
ALTER TABLE attempts ADD COLUMN dispatch_lease_until timestamptz;
ALTER TABLE attempts ADD CONSTRAINT dispatch_claim_complete CHECK(
    (dispatch_owner_id IS NULL AND dispatch_fence IS NULL AND dispatch_lease_until IS NULL)
    OR (dispatch_owner_id IS NOT NULL AND dispatch_fence IS NOT NULL AND dispatch_lease_until IS NOT NULL)
);
CREATE FUNCTION preserve_dispatch_claim() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.dispatch_fence IS NOT NULL AND (
       NEW.dispatch_fence IS DISTINCT FROM OLD.dispatch_fence OR
       NEW.dispatch_owner_id IS DISTINCT FROM OLD.dispatch_owner_id OR
       NEW.dispatch_lease_until IS DISTINCT FROM OLD.dispatch_lease_until) THEN
        RAISE EXCEPTION 'dispatch claim is immutable' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER dispatch_claim_immutable BEFORE UPDATE ON attempts FOR EACH ROW EXECUTE FUNCTION preserve_dispatch_claim();
CREATE INDEX attempts_expired_dispatch ON attempts(dispatch_lease_until) WHERE state IN ('dispatched','streaming');
