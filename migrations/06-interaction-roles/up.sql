CREATE TABLE interaction_role (
    guild_id INTEGER NOT NULL, -- Snowflake/u64 --

    name TEXT NOT NULL,
    description TEXT NULL,
    channel_id INTEGER NOT NULL, -- Snowflake/u64 --
    message_id INTEGER NULL, -- Snowflake/u64 --
    exclusive INTEGER NOT NULL DEFAULT(FALSE),
    
    last_updated TEXT NOT NULL DEFAULT('1970-01-01T01:00:00+00:00'),

    PRIMARY KEY (guild_id, name)
) STRICT;

CREATE TABLE interaction_role_choice (
    guild_id INTEGER NOT NULL, -- Snowflake/u64 --
    set_name TEXT NOT NULL,

    choice TEXT NOT NULL,
    emoji TEXT,
    role_id INTEGER NOT NULL, -- Snowflake/u64 --

    last_updated TEXT NOT NULL DEFAULT('1970-01-01T01:00:00+00:00'),

    PRIMARY KEY (guild_id, set_name, choice)
) STRICT;