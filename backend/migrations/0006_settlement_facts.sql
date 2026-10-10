CREATE TABLE settlement_facts (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    attempt_id uuid NOT NULL,
    source text NOT NULL CHECK(length(source) BETWEEN 1 AND 256),
    source_event_id text NOT NULL CHECK(length(source_event_id) BETWEEN 1 AND 256),
    fact jsonb NOT NULL CHECK(jsonb_typeof(fact)='object'),
    ledger_id uuid NOT NULL,
    result_amount numeric(38,18) CHECK(result_amount>=0),
    result_confidence text NOT NULL CHECK(result_confidence IN ('confirmed','estimated','unknown')),
    CHECK((result_confidence='unknown')=(result_amount IS NULL)),
    observed_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY(installation_id,attempt_id) REFERENCES attempts(installation_id,id),
    FOREIGN KEY(installation_id,ledger_id) REFERENCES ledger_entries(installation_id,id),
    UNIQUE(installation_id,source,source_event_id)
);
CREATE TRIGGER settlement_fact_immutable BEFORE UPDATE OR DELETE ON settlement_facts FOR EACH ROW EXECUTE FUNCTION forbid_immutable_change();
-- Rebuildable effective pointer. Original usage/ledger/receipts remain append-only.
CREATE TABLE attempt_expenses (
    installation_id uuid NOT NULL REFERENCES installations(id),
    attempt_id uuid PRIMARY KEY,
    ledger_id uuid NOT NULL,
    amount numeric(38,18) CHECK(amount>=0),
    currency varchar(3) NOT NULL CHECK(currency ~ '^[A-Z]{3}$'),
    confidence text NOT NULL CHECK(confidence IN ('confirmed','estimated','unknown')),
    version bigint NOT NULL DEFAULT 1 CHECK(version>0),
    CHECK((confidence='unknown')=(amount IS NULL)),
    FOREIGN KEY(installation_id,attempt_id) REFERENCES attempts(installation_id,id),
    FOREIGN KEY(installation_id,ledger_id) REFERENCES ledger_entries(installation_id,id)
);
