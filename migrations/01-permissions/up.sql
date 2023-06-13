CREATE TABLE permission (
    guild_id INTEGER NOT NULL, -- Snowflake/u64 --

    discord_id INTEGER NOT NULL, -- Snowflake/u64 --
    type TEXT NOT NULL
        CHECK(type IN ('ROLE', 'USER')),
        
    value TEXT NOT NULL,

    last_updated TEXT NOT NULL DEFAULT('1970-01-01T01:00:00+00:00'),

    PRIMARY KEY (guild_id, discord_id, type, value)
) STRICT;