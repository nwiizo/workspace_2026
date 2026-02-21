-- Posts table
CREATE TABLE IF NOT EXISTS posts (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    reply_to_id UUID REFERENCES posts(id) ON DELETE SET NULL,
    reply_count INT NOT NULL DEFAULT 0,
    like_count INT NOT NULL DEFAULT 0,
    rechirp_count INT NOT NULL DEFAULT 0,
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Partial index for non-deleted posts (most queries filter by this)
CREATE INDEX idx_posts_active ON posts (created_at DESC) WHERE is_deleted = FALSE;
CREATE INDEX idx_posts_user ON posts (user_id, created_at DESC) WHERE is_deleted = FALSE;
CREATE INDEX idx_posts_reply_to ON posts (reply_to_id, created_at DESC) WHERE is_deleted = FALSE;

CREATE TRIGGER update_posts_updated_at
    BEFORE UPDATE ON posts
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
