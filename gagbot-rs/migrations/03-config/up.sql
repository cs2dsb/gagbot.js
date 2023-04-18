CREATE TABLE config (
    guild_id INTEGER NOT NULL, -- Snowflake/u64 --

    key TEXT NOT NULL,
    value TEXT NOT NULL,

    last_updated TEXT NOT NULL DEFAULT('1970-01-01T01:00:00+00:00'),

    PRIMARY KEY (guild_id, key)
) STRICT;