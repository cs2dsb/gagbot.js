-- These are a direct migration from mongo --
CREATE TABLE reaction_role_temp (
    guild_id INTEGER NOT NULL, -- Snowflake/u64 --

    exclusive INTEGER NOT NULL DEFAULT(FALSE),

    name TEXT NOT NULL,
    channel_id INTEGER NOT NULL, -- Snowflake/u64 --
    message_id INTEGER NOT NULL, -- Snowflake/u64 --
    
    last_updated TEXT NOT NULL DEFAULT('1970-01-01T01:00:00+00:00'),

    PRIMARY KEY (guild_id, name)
) STRICT;

CREATE TABLE reaction_role_choice_temp (
    guild_id INTEGER NOT NULL, -- Snowflake/u64 --
    set_name TEXT NOT NULL,

    choice TEXT NOT NULL,
    role_id INTEGER NOT NULL, -- Snowflake/u64 --

    last_updated TEXT NOT NULL DEFAULT('1970-01-01T01:00:00+00:00'),

    PRIMARY KEY (guild_id, set_name, choice)
) STRICT;