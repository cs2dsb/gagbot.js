pub mod promote;
pub mod add_member;
pub mod greet;
pub mod log;

#[macro_export]
macro_rules! get_config_string_option {
    ($data:expr, $guild_id: expr, $key:expr) => {{
        use anyhow::Context as _;
        $data
            .get_config_string($guild_id, $key)
            .await
            .with_context(|| format!("Failed to get {} config value", $key))?
    }};
}

#[macro_export]
macro_rules! get_config_string {
    ($data:expr, $guild_id: expr, $key:expr) => {{
        let value = crate::get_config_string_option!($data, $guild_id, $key);

        if value.is_none() {
            return Ok(OptionallyConfiguredResult::Unconfigured($key));
        }

        value.unwrap()
    }};
}

#[macro_export]
macro_rules! get_config_chan_option {
    ($ctx:expr, $data:expr, $guild_id: expr, $key:expr) => {{
        use anyhow::Context as _;
        use poise::serenity_prelude::{ ChannelId, Channel, ChannelType };
        use std::str::FromStr;
        let string = crate::get_config_string_option!($data, $guild_id, $key);
        if let Some(string) = string {
            let id = ChannelId::from_str(&string)
                .with_context(|| format!("Failed to parse {} ({}) as a ChannelId", $key, string))?;

            let chan = id
                .to_channel($ctx)
                .await
                .with_context(|| format!("Failed to resolve {} ({:?}) to a channel", $key, id))?;

            let chan = if let Channel::Guild(c) = chan {
                if c.kind == ChannelType::Text {
                    Some(c)
                } else {
                    None
                }
            } else {
                None
            };

            Some(chan.ok_or(anyhow::anyhow!(
                "Channel for {} must be a text channel",
                $key
            ))?)
        } else {
            None
        }
    }};
}

#[macro_export]
macro_rules! get_config_chan {
    ($ctx:expr, $data:expr, $guild_id: expr, $key:expr) => {{
        let value = crate::get_config_chan_option!($ctx, $data, $guild_id, $key);
        if value.is_none() {
            return Ok(OptionallyConfiguredResult::Unconfigured($key));
        }    
        value.unwrap()
    }};
}

#[macro_export]
macro_rules! get_config_role_option {
    ($ctx:expr, $data:expr, $guild_id: expr, $key:expr) => {{
        use anyhow::Context as _;
        use poise::serenity_prelude::{ RoleId };
        use std::str::FromStr;
        let string = crate::get_config_string_option!($data, $guild_id, $key);
        if let Some(string) = string {
            let id = RoleId::from_str(&string)
                .with_context(|| format!("Failed to parse {} ({}) as a RoleId", $key, string))?;

            Some(if let Some(role) = id.to_role_cached($ctx) {
                role
            } else {
                // This should warm up the cache only on the first miss
                $guild_id
                    .roles($ctx)
                    .await
                    .context("Failed to lookup guild roles")?;

                id.to_role_cached($ctx).ok_or(anyhow::anyhow!(
                    "Failed to resolve {} ({:?}) to a role",
                    $key,
                    id
                ))?
            })
        } else {
            None
        }
    }};
}

#[macro_export]
macro_rules! get_config_role {
    ($ctx:expr, $data:expr, $guild_id: expr, $key:expr) => {{
        let value = crate::get_config_role_option!($ctx, $data, $guild_id, $key);
        if value.is_none() {
            return Ok(OptionallyConfiguredResult::Unconfigured($key));
        }    
        value.unwrap()
    }};
}

#[macro_export]
macro_rules! get_config_u64_option {
    ($data:expr, $guild_id:expr, $key:expr) => {{
        use anyhow::Context as _;
        use std::str::FromStr;
        let string = crate::get_config_string_option!($data, $guild_id, $key);
        if let Some(string) = string {
            Some(u64::from_str(&string).with_context(|| {
                format!("Failed to parse {} ({}) as an unsigned int", $key, string)
            })?)
        } else {
            None
        }
    }};
}

#[macro_export]
macro_rules! get_config_u64 {
    ($data:expr, $guild_id:expr, $key:expr) => {{
        let value = crate::get_config_u64_option!($data, $guild_id, $key);
        if value.is_none() {
            return Ok(OptionallyConfiguredResult::Unconfigured($key));
        }    
        value.unwrap()
    }};
}