CREATE TYPE ip_status AS ENUM ('available', 'assigned', 'reserved', 'deprecated');

CREATE TABLE ip_addresses (
    id UUID PRIMARY KEY,
    address INET NOT NULL,
    subnet_id UUID NOT NULL REFERENCES subnets(id) ON DELETE CASCADE,
    status ip_status NOT NULL DEFAULT 'available',
    hostname VARCHAR(255),
    assigned_to VARCHAR(255),
    description TEXT,
    mac_address VARCHAR(17),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(address)
);

CREATE INDEX idx_ip_addresses_subnet_id ON ip_addresses(subnet_id);
CREATE INDEX idx_ip_addresses_status ON ip_addresses(status);
CREATE INDEX idx_ip_addresses_address ON ip_addresses USING gist (address inet_ops);
