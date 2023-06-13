CREATE TABLE message_count (
    guild_id INTEGER NOT NULL, -- Snowflake/u64 --
    user_id INTEGER NOT NULL, -- Snowflake/u64 --
    channel_id INTEGER NOT NULL, -- Snowflake/u64 --
    message_count INTEGER NOT NULL DEFAULT(0),

    PRIMARY KEY (guild_id, user_id, channel_id)
) STRICT;