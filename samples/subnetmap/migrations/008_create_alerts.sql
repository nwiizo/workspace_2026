CREATE TYPE alert_type AS ENUM ('high_utilization', 'duplicate_ip', 'subnet_full');

CREATE TABLE alerts (
    id UUID PRIMARY KEY,
    alert_type alert_type NOT NULL,
    subnet_id UUID REFERENCES subnets(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    is_acknowledged BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alerts_unacknowledged ON alerts(is_acknowledged) WHERE NOT is_acknowledged;
