-- Auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_subnets_updated_at
    BEFORE UPDATE ON subnets
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trigger_ip_addresses_updated_at
    BEFORE UPDATE ON ip_addresses
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trigger_vlans_updated_at
    BEFORE UPDATE ON vlans
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trigger_dns_records_updated_at
    BEFORE UPDATE ON dns_records
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
