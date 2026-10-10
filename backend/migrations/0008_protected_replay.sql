-- Existing requests have no qualified wire mode; do not guess it on upgrade.
ALTER TABLE requests ADD COLUMN wire_protocol text CHECK(wire_protocol IN ('chat_completions','responses'));
ALTER TABLE requests ADD COLUMN streaming boolean;
ALTER TABLE requests ADD CONSTRAINT request_wire_mode_complete CHECK((wire_protocol IS NULL)=(streaming IS NULL));
CREATE TRIGGER request_wire_mode_immutable BEFORE UPDATE OF wire_protocol,streaming ON requests FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
CREATE TABLE replay_receipts (
    request_id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    protocol text NOT NULL CHECK(protocol IN ('chat_completions','responses')),
    status_code integer NOT NULL CHECK(status_code BETWEEN 200 AND 599),
    body_binding bytea NOT NULL CHECK(octet_length(body_binding)=32),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    CHECK(expires_at>created_at AND expires_at<=created_at+interval '24 hours'),
    FOREIGN KEY(installation_id,request_id) REFERENCES requests(installation_id,id)
);
CREATE TRIGGER replay_receipt_immutable BEFORE UPDATE OR DELETE ON replay_receipts FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
-- Legacy ciphertext stays intact but is not guessed into a qualified result receipt.
ALTER TABLE replay_payloads ADD CONSTRAINT replay_payload_receipt FOREIGN KEY(request_id) REFERENCES replay_receipts(request_id) NOT VALID;
CREATE TRIGGER replay_ciphertext_immutable BEFORE UPDATE ON replay_payloads FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
ALTER TABLE replay_payloads ADD CONSTRAINT replay_ciphertext_size CHECK(octet_length(encrypted_response)<=2097168) NOT VALID;
CREATE INDEX replay_expiry ON replay_receipts(installation_id,expires_at,request_id);
