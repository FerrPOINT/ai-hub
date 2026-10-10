-- No invented source ID/version on legacy requests. New admission records a real resolution.
ALTER TABLE requests ADD COLUMN pricing_source_revision_id uuid;
ALTER TABLE requests ADD COLUMN pricing_policy_version bigint CHECK(pricing_policy_version>=0);
ALTER TABLE requests ADD CONSTRAINT request_source_version CHECK(pricing_source_revision_id IS NULL OR pricing_policy_version IS NOT NULL AND pricing_policy_version>0);
ALTER TABLE requests ADD CONSTRAINT request_source_fk FOREIGN KEY(installation_id,pricing_source_revision_id) REFERENCES pricing_source_revisions(installation_id,id);
CREATE TRIGGER request_pricing_source_immutable BEFORE UPDATE OF pricing_source_revision_id,pricing_policy_version ON requests FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
