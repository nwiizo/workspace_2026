-- Post media attachments
CREATE TABLE IF NOT EXISTS post_media (
    id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    media_url TEXT NOT NULL,
    media_type VARCHAR(50) NOT NULL DEFAULT 'image/jpeg',
    width INT,
    height INT,
    position SMALLINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_post_media_post ON post_media (post_id, position);
