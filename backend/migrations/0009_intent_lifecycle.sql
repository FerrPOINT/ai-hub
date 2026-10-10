-- A bounded queue lifetime is recorded before I/O. Legacy intents cannot acquire new claims.
ALTER TABLE requests ADD COLUMN intent_ttl_seconds integer CHECK(intent_ttl_seconds BETWEEN 5 AND 120);
ALTER TABLE requests ADD COLUMN intent_deadline timestamptz;
ALTER TABLE requests ADD CONSTRAINT bounded_intent CHECK(
    (intent_deadline IS NULL AND intent_ttl_seconds IS NULL)
    OR (intent_deadline IS NOT NULL AND intent_ttl_seconds IS NOT NULL
        AND intent_deadline>admitted_at AND intent_deadline<=admitted_at+interval '120 seconds')
);
CREATE TRIGGER request_intent_deadline_immutable BEFORE UPDATE OF intent_deadline,intent_ttl_seconds ON requests FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE INDEX unclaimed_intent_expiry ON requests(installation_id,intent_deadline,id) WHERE state='admitted';
