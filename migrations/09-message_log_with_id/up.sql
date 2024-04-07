-- This table is identical to the message_log table but with an auto id added
CREATE TABLE message_log_with_id (
    message_index_id INTEGER PRIMARY KEY,
    guild_id INTEGER NOT NULL, -- Snowflake/u64 --
    user_id INTEGER, -- Snowflake/u64 --
    channel_id INTEGER NOT NULL, -- Snowflake/u64 --
    message_id INTEGER NOT NULL, -- Snowflake/u64 --

    timestamp TEXT NOT NULL,

    type TEXT NOT NULL
        CHECK(type IN ('CREATE', 'EDIT', 'DELETE', 'PURGE')),

    message_json TEXT
) STRICT;

INSERT INTO message_log_with_id (guild_id, user_id, channel_id, message_id, timestamp, type, message_json)
SELECT guild_id, user_id, channel_id, message_id, timestamp, type, message_json FROM message_log;

DROP TABLE message_log;
ALTER TABLE message_log_with_id RENAME TO message_log;