CREATE TRIGGER credential_ciphertext_immutable BEFORE UPDATE OF connection_id,generation,ciphertext,nonce,key_id,created_at OR DELETE ON credential_versions FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
ALTER TABLE credential_versions ADD CONSTRAINT credential_ciphertext_bound CHECK(octet_length(ciphertext)<=16400) NOT VALID;
