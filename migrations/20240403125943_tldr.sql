-- Add migration script here
CREATE TABLE chat_messages (
    chat_id    INTEGER NOT NULL,
    user_id    INTEGER NOT NULL,
    message_id INTEGER NOT NULL,
    message    TEXT,
    created_at REAL DEFAULT current_timestamp,

    PRIMARY KEY (chat_id, user_id, message_id)
);