-- pg_trgm for fuzzy text search
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX idx_subnets_name_trgm ON subnets USING gin (name gin_trgm_ops);
CREATE INDEX idx_ip_addresses_hostname_trgm ON ip_addresses USING gin (hostname gin_trgm_ops);
CREATE INDEX idx_ip_addresses_assigned_to_trgm ON ip_addresses USING gin (assigned_to gin_trgm_ops);
CREATE INDEX idx_vlans_name_trgm ON vlans USING gin (name gin_trgm_ops);
CREATE INDEX idx_dns_records_hostname_trgm ON dns_records USING gin (hostname gin_trgm_ops);
