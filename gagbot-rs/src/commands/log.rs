use poise::serenity_prelude::{Cache, CacheHttp, Http};

use crate::{
    db::queries::config::{LogChannel},
    BotData, GuildId, Embed,
};


use super::promote::OptionallyConfiguredResult;


pub async fn log<'a, 'b, T>(
    data: &'a BotData,
    ctx: &'a T,
    guild_id: GuildId,
    log_channel: Vec<LogChannel>,
    embed: Embed,
) -> anyhow::Result<OptionallyConfiguredResult<()>>
where
T: 'a + Clone + CacheHttp + AsRef<Cache> + AsRef<Http>,
{
    assert!(log_channel.len() > 0, "log called with no channels");
    
    // Only used for displaying unconfigured notice
    let first_config_key = log_channel[0].into();

    if let Some(channel_id) = data
        .log_channel(guild_id.into(), log_channel)
        .await?
    {
        embed
            .send_in_channel(channel_id, &ctx)
            .await?;

        Ok(OptionallyConfiguredResult::Ok(()))
    } else {
        Ok(OptionallyConfiguredResult::Unconfigured(first_config_key))
    }
}
