CREATE TABLE message_log (
    guild_id INTEGER NOT NULL, -- Snowflake/u64 --
    user_id INTEGER, -- Snowflake/u64 --
    channel_id INTEGER NOT NULL, -- Snowflake/u64 --
    message_id INTEGER NOT NULL, -- Snowflake/u64 --

    timestamp TEXT NOT NULL,

    type TEXT NOT NULL
        CHECK(type IN ('CREATE', 'EDIT', 'DELETE')),

    content TEXT
) STRICT;