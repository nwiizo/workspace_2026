CREATE TABLE tags (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    color VARCHAR(7) NOT NULL DEFAULT '#3b82f6',
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE ip_tags (
    ip_address_id UUID NOT NULL REFERENCES ip_addresses(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (ip_address_id, tag_id)
);
