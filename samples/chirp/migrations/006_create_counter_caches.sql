-- Counter cache triggers for denormalized counts

-- Like count trigger
CREATE OR REPLACE FUNCTION update_like_count() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE posts SET like_count = like_count + 1 WHERE id = NEW.post_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE posts SET like_count = like_count - 1 WHERE id = OLD.post_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_like_count
    AFTER INSERT OR DELETE ON likes
    FOR EACH ROW EXECUTE FUNCTION update_like_count();

-- Rechirp count trigger
CREATE OR REPLACE FUNCTION update_rechirp_count() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE posts SET rechirp_count = rechirp_count + 1 WHERE id = NEW.post_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE posts SET rechirp_count = rechirp_count - 1 WHERE id = OLD.post_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_rechirp_count
    AFTER INSERT OR DELETE ON rechirps
    FOR EACH ROW EXECUTE FUNCTION update_rechirp_count();

-- Reply count trigger
CREATE OR REPLACE FUNCTION update_reply_count() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' AND NEW.reply_to_id IS NOT NULL THEN
        UPDATE posts SET reply_count = reply_count + 1 WHERE id = NEW.reply_to_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' AND OLD.reply_to_id IS NOT NULL THEN
        UPDATE posts SET reply_count = reply_count - 1 WHERE id = OLD.reply_to_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_reply_count
    AFTER INSERT OR DELETE ON posts
    FOR EACH ROW EXECUTE FUNCTION update_reply_count();

-- Follower/following count triggers
CREATE OR REPLACE FUNCTION update_follow_counts() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE users SET following_count = following_count + 1 WHERE id = NEW.follower_id;
        UPDATE users SET followers_count = followers_count + 1 WHERE id = NEW.following_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE users SET following_count = following_count - 1 WHERE id = OLD.follower_id;
        UPDATE users SET followers_count = followers_count - 1 WHERE id = OLD.following_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_follow_counts
    AFTER INSERT OR DELETE ON follows
    FOR EACH ROW EXECUTE FUNCTION update_follow_counts();

-- Post count trigger
CREATE OR REPLACE FUNCTION update_posts_count() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE users SET posts_count = posts_count + 1 WHERE id = NEW.user_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE users SET posts_count = posts_count - 1 WHERE id = OLD.user_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_posts_count
    AFTER INSERT OR DELETE ON posts
    FOR EACH ROW EXECUTE FUNCTION update_posts_count();
