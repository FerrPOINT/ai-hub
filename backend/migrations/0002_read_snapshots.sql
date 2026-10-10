-- Bounded, expiring authorised projections. Opaque cursors carry no authority.
CREATE TABLE read_snapshots (
    id uuid PRIMARY KEY,
    installation_id uuid NOT NULL REFERENCES installations(id),
    subject text NOT NULL CHECK (length(subject) BETWEEN 1 AND 256),
    query_identity text NOT NULL CHECK (length(query_identity) BETWEEN 1 AND 1024),
    items jsonb NOT NULL CHECK (jsonb_typeof(items)='array' AND jsonb_array_length(items)<=10000),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL DEFAULT now()+interval '15 minutes',
    CHECK (expires_at>created_at AND expires_at<=created_at+interval '15 minutes')
);
CREATE TABLE read_cursors (
    id uuid PRIMARY KEY,
    snapshot_id uuid NOT NULL REFERENCES read_snapshots(id) ON DELETE CASCADE,
    position integer NOT NULL CHECK (position BETWEEN 1 AND 10000),
    UNIQUE(snapshot_id,position)
);
CREATE INDEX read_snapshots_expiry ON read_snapshots(expires_at);
