-- Notification event type
CREATE TYPE notification_event AS ENUM ('like', 'rechirp', 'follow', 'reply', 'mention');

-- Notifications
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    actor_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_type notification_event NOT NULL,
    post_id UUID REFERENCES posts(id) ON DELETE CASCADE,
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_user ON notifications (user_id, created_at DESC);
CREATE INDEX idx_notifications_unread ON notifications (user_id, is_read) WHERE is_read = FALSE;
