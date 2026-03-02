CREATE TYPE dns_record_type AS ENUM ('A', 'AAAA', 'PTR');

CREATE TABLE dns_records (
    id UUID PRIMARY KEY,
    record_type dns_record_type NOT NULL DEFAULT 'A',
    hostname VARCHAR(255) NOT NULL,
    ip_address_id UUID NOT NULL REFERENCES ip_addresses(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_dns_records_ip_address_id ON dns_records(ip_address_id);
CREATE INDEX idx_dns_records_hostname ON dns_records(hostname);
