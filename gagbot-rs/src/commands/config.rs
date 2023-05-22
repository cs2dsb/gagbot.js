use std::str::FromStr;
use std::fmt::Write;
use poise::{self, serenity_prelude::ChannelId, SlashArgument};
use tokio::sync::oneshot;

use crate::{Context, DbCommand, Embed, Error, config::ConfigKey, permissions::{Permission, PermissionCheck}};

#[poise::command(prefix_command, slash_command, guild_only, category = "Config")]
/// Print the current value of the provided config key
pub async fn get_config(
    ctx: Context<'_>,

    #[description = "The config key you want to look up the value for"] key: ConfigKey,
) -> Result<(), Error> {
    ctx.require_permission(Permission::ConfigManage).await?;

    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");

    let (s, r) = oneshot::channel();
    ctx.data()
        .db_command_sender
        .send_async(DbCommand::GetConfigString {
            guild_id: guild_id.into(),
            key,
            respond_to: s,
        })
        .await?;

    let value = r.await?;

    let (msg, err) = match value {
        Ok(None) => (format!("{} is not set", key), false),
        Ok(Some(v)) => (format!("{} = {}", key, v), false),
        Err(e) => (format!("Error fetching {}: {:?}", key, e), true),
    };

    Embed::default()
        .description(msg)
        .set_error(err)
        .send(&ctx)
        .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Config")]
/// Set the config for the provided key with the provided value
pub async fn set_config(
    ctx: Context<'_>,

    #[description = "The config key you want to set the value for"] key: ConfigKey,
    #[description = "The value you want to change it to"] value: String,
) -> Result<(), Error> {
    ctx.require_permission(Permission::ConfigManage).await?;

    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");
    let timestamp = ctx.created_at();

    let (s, r) = oneshot::channel();
    ctx.data()
        .db_command_sender
        .send_async(DbCommand::SetConfigString {
            guild_id: guild_id.into(),
            key,
            value: value,
            timestamp,
            respond_to: s,
        })
        .await?;

    let value = r.await?;

    let (msg, err) = match value {
        Ok(()) => (format!("{} changed", key), false),
        Err(e) => (format!("Error setting {}: {:?}", key, e), true),
    };

    Embed::default()
        .description(msg)
        .set_error(err)
        .send(&ctx)
        .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Config")]
/// Set all logging types to a single provided channel
pub async fn set_log(
    ctx: Context<'_>,

    #[description = "The channel you want all logs to go to"] channel: ChannelId,
) -> Result<(), Error> {
    ctx.require_permission(Permission::ConfigManage).await?;

    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");
    let timestamp = ctx.created_at();

    let mut msg = String::new();
    let mut is_error = false;
    for key in ConfigKey::logging_keys().iter() {
        let r = ctx.data().set_config(
            guild_id.into(),
            *key,
            timestamp,
            channel.to_string(),
        ).await;
        if msg.len() > 0 {
            msg.push('\n');
        }
        msg.push_str(&match r {
            Ok(()) => format!("{} changed", key),
            Err(e) => {
                is_error = true;
                format!("Error setting {}: {:?}", key, e)
            },
        });
    }

    Embed::default()
        .description(msg)
        .set_error(is_error)
        .send(&ctx)
        .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Config")]
/// Delete the config for the provided key
pub async fn delete_config(
    ctx: Context<'_>,

    #[description = "The config key you want to delete the value for"] key: ConfigKey,
) -> Result<(), Error> {
    ctx.require_permission(Permission::ConfigManage).await?;

    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");
    let timestamp = ctx.created_at();

    let (s, r) = oneshot::channel();
    ctx.data()
        .db_command_sender
        .send_async(DbCommand::DeleteConfig {
            guild_id: guild_id.into(),
            key,
            timestamp,
            respond_to: s,
        })
        .await?;

    let value = r.await?;

    let (msg, err) = match value {
        Ok(()) => (format!("{} deleted", key), false),
        Err(e) => (format!("Error deleting {}: {:?}", key, e), true),
    };

    Embed::default()
        .description(msg)
        .set_error(err)
        .send(&ctx)
        .await?;

    Ok(())
}
#[poise::command(prefix_command, slash_command, guild_only, category = "Config")]
/// Get help on config flags
pub async fn config_help(
    ctx: Context<'_>,

    #[description = "The config key you want help with"] key: Option<ConfigKey>,
) -> Result<(), Error> {
    ctx.require_permission(Permission::ConfigManage).await?;

    let mut keys = Vec::new();

    if let Some(key) = key {
        keys.push(key);
    } else {
        for choice in ConfigKey::choices() {
            keys.push(ConfigKey::from_str(&choice.name)?)
        }
    }

    let mut msg = String::new();
    for keys in keys {
        write!(&mut msg, "**{}**\n", keys.name())?;
        write!(&mut msg, "{}\n\n", keys.description())?;
    }

    Embed::success()
        .title("Config help")
        .description(msg)
        .send(&ctx)
        .await?;


    Ok(())
}