use anyhow::Context as AnyhowContext;
use poise::serenity_prelude::{Cache, CacheHttp, Http, Member};

use crate::{
    get_config_string, get_config_role, get_config_chan,
    db::queries::config::{ConfigKey, LogChannel},
    with_progress_embed, BotData, GuildId, expand_greeting_template, Embed,
};


use super::promote::OptionallyConfiguredResult;

pub async fn run_add_member<'a, 'b, T>(
    data: &'a BotData,
    ctx: &'a T,
    guild_id: GuildId,
    member: Member,
) -> anyhow::Result<OptionallyConfiguredResult<()>>
where
    T: 'a + Clone + CacheHttp + AsRef<Cache> + AsRef<Http>,
{
    const PROGRESS_TITLE: &str = "Adding member";

    async fn work<'a, Ctx>(
        ctx: &'a Ctx,
        (guild_id, data, mut member): (GuildId, &'a BotData, Member),
        progress_chan: flume::Sender<String>,
    ) -> anyhow::Result<OptionallyConfiguredResult<()>>
    where
        Ctx: 'a + CacheHttp + AsRef<Http> + AsRef<Cache>,
    {
        // Get all the config we will need
        let new_role = get_config_role!(ctx, data, guild_id, ConfigKey::GreetRole);
        let default_role = get_config_role!(ctx, data, guild_id, ConfigKey::GreetDefaultRole);
        
        let welcome_channel = get_config_chan!(ctx, data, guild_id, ConfigKey::GreetWelcomeChannel);
        let mut welcome_message = get_config_string!(data, guild_id, ConfigKey::GreetWelcomeMessage);
        
        // Add the new role first 
        if !member.roles.contains(&new_role.id) {
            progress_chan
                .send_async(format!("Promoting {} to {}", member, new_role))
                .await?;
            member.add_role(ctx, new_role.id)
                .await
                .context("Adding new role")?;
        }

        // Remove the old role
        if member.roles.contains(&default_role.id) {
            progress_chan
                .send_async(format!("Removing {} from {}", default_role, member))
                .await?;
            member.remove_role(ctx, default_role.id)
                .await
                .context("Removing default role")?;
        }

        // Post the welcome message
        progress_chan
            .send_async(format!("Welcoming {}", member))
            .await?;
        expand_greeting_template(&member.user, &mut welcome_message);
        Embed::default()
            .content(format!("{member}"))
            .description(welcome_message)
            .random_color()
            .send_in_channel(welcome_channel.id.into(), ctx)
            .await?;

        Ok(OptionallyConfiguredResult::Ok(()))
    }

    with_progress_embed(
        data,
        ctx,
        guild_id,
        LogChannel::General,
        PROGRESS_TITLE,
        work,
        (guild_id, data, member),
    )
    .await
}
