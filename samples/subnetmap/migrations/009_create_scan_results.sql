CREATE TABLE scan_results (
    id UUID PRIMARY KEY,
    subnet_id UUID NOT NULL REFERENCES subnets(id) ON DELETE CASCADE,
    ip_address INET NOT NULL,
    is_alive BOOLEAN NOT NULL DEFAULT FALSE,
    last_seen TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_scan_results_subnet_id ON scan_results(subnet_id);
CREATE UNIQUE INDEX idx_scan_results_unique_ip ON scan_results(subnet_id, ip_address);
