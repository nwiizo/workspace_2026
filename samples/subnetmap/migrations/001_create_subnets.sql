CREATE TABLE subnets (
    id UUID PRIMARY KEY,
    cidr CIDR NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    parent_id UUID REFERENCES subnets(id) ON DELETE SET NULL,
    used_count INTEGER NOT NULL DEFAULT 0,
    total_addresses INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_subnets_parent_id ON subnets(parent_id);
CREATE INDEX idx_subnets_cidr ON subnets USING gist (cidr inet_ops);
