-- Enable pg_trgm extension for trigram-based text search (Japanese-friendly)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Trigram indexes for fuzzy text search
CREATE INDEX idx_posts_content_trgm ON posts USING gin (content gin_trgm_ops) WHERE is_deleted = FALSE;
CREATE INDEX idx_users_username_trgm ON users USING gin (username gin_trgm_ops);
CREATE INDEX idx_users_display_name_trgm ON users USING gin (display_name gin_trgm_ops);
