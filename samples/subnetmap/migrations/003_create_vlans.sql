CREATE TABLE vlans (
    id UUID PRIMARY KEY,
    vlan_id INTEGER NOT NULL CHECK (vlan_id >= 1 AND vlan_id <= 4094) UNIQUE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE vlan_subnet_links (
    vlan_id UUID NOT NULL REFERENCES vlans(id) ON DELETE CASCADE,
    subnet_id UUID NOT NULL REFERENCES subnets(id) ON DELETE CASCADE,
    PRIMARY KEY (vlan_id, subnet_id)
);
