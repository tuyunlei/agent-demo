-- Events table (append-only event stream)
CREATE TABLE IF NOT EXISTS events (
    event_id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id),
    sequence_number BIGINT NOT NULL,
    event_type VARCHAR(64) NOT NULL,
    payload JSONB NOT NULL,
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(session_id, sequence_number)
);

CREATE INDEX idx_events_session_seq_desc ON events(session_id, sequence_number DESC);
