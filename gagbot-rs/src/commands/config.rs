use poise::{self, serenity_prelude::ChannelId};
use tokio::sync::oneshot;

use crate::{config::ConfigKey, Context, DbCommand, Error};

#[poise::command(prefix_command, slash_command, guild_only, category = "Config")]
/// Print the current value of the provided config key
pub async fn get_config(
    ctx: Context<'_>,

    #[description = "The config key you want to look up the value for"] key: ConfigKey,
) -> Result<(), Error> {
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

    ctx.say(match value {
        Ok(None) => format!("{} is not set", key),
        Ok(Some(v)) => format!("{} = {}", key, v),
        Err(e) => format!("Error fetching {}: {:?}", key, e),
    })
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

    ctx.say(match value {
        Ok(()) => format!("{} changed", key),
        Err(e) => format!("Error setting {}: {:?}", key, e),
    })
    .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Config")]
/// Set all log config keys to the provided value
pub async fn set_log(
    ctx: Context<'_>,

    #[description = "The value you want to change it to"] value: ChannelId,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");
    let timestamp = ctx.created_at();

    let mut msg = String::new();

    for key in ConfigKey::logging_keys().iter() {
        let r = ctx.data().set_config(
            guild_id.into(),
            *key,
            timestamp,
            value.to_string(),
        ).await;
        if msg.len() > 0 {
            msg.push('\n');
        }
        msg.push_str(&match r {
            Ok(()) => format!("{} changed", key),
            Err(e) => format!("Error setting {}: {:?}", key, e),
        });
    }
    ctx.say(msg).await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Config")]
/// Delete the config for the provided key
pub async fn delete_config(
    ctx: Context<'_>,

    #[description = "The config key you want to delete the value for"] key: ConfigKey,
) -> Result<(), Error> {
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

    ctx.say(match value {
        Ok(()) => format!("{} deleted", key),
        Err(e) => format!("Error deleting {}: {:?}", key, e),
    })
    .await?;

    Ok(())
}
