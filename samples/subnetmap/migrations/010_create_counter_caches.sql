-- Counter cache trigger: update subnets.used_count when ip_addresses change

CREATE OR REPLACE FUNCTION update_subnet_used_count() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.status != 'available' THEN
            UPDATE subnets SET used_count = used_count + 1 WHERE id = NEW.subnet_id;
        END IF;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        IF OLD.status != 'available' THEN
            UPDATE subnets SET used_count = used_count - 1 WHERE id = OLD.subnet_id;
        END IF;
        RETURN OLD;
    ELSIF TG_OP = 'UPDATE' THEN
        IF OLD.status = 'available' AND NEW.status != 'available' THEN
            UPDATE subnets SET used_count = used_count + 1 WHERE id = NEW.subnet_id;
        ELSIF OLD.status != 'available' AND NEW.status = 'available' THEN
            UPDATE subnets SET used_count = used_count - 1 WHERE id = NEW.subnet_id;
        END IF;
        RETURN NEW;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_subnet_used_count
    AFTER INSERT OR UPDATE OR DELETE ON ip_addresses
    FOR EACH ROW EXECUTE FUNCTION update_subnet_used_count();
