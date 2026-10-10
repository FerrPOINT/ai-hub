-- Two registries may reference the same Tracker UUID. Identity is the binding, never the label.
DO $$
DECLARE old_constraint text;
BEGIN
    SELECT conname INTO STRICT old_constraint FROM pg_constraint
    WHERE conrelid='budget_policies'::regclass AND contype='u'
      AND pg_get_constraintdef(oid)='UNIQUE (installation_id, scope_type, scope_id, currency, period)';
    EXECUTE format('ALTER TABLE budget_policies DROP CONSTRAINT %I',old_constraint);
END;
$$;
ALTER TABLE budget_policies ADD CONSTRAINT budget_scope_currency_period
    UNIQUE NULLS NOT DISTINCT(installation_id,scope_type,scope_id,namespace_binding_id,currency,period);
CREATE TRIGGER budget_identity_immutable BEFORE UPDATE OF installation_id,scope_type,scope_id,namespace_binding_id,currency,period OR DELETE ON budget_policies FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
