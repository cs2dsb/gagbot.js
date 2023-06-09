use anyhow::Context as AnyhowContext;
use poise::serenity_prelude::{Cache, CacheHttp,   Http, Member};

use crate::{
    get_config_string, get_config_chan,
    db::queries::config::{ConfigKey},
    BotData,  GuildId,  expand_greeting_template, Embed, get_config_role_option,
};


use super::promote::OptionallyConfiguredResult;

pub async fn run_greet<'a, 'b, T>(
    data: &'a BotData,
    ctx: &'a T,
    guild_id: GuildId,
    mut member: Member,
    add_role: bool,
) -> anyhow::Result<OptionallyConfiguredResult<()>>
where
    T: 'a + Clone + CacheHttp + AsRef<Cache> + AsRef<Http>,
{
    // Get all the config we will need
    let mut greet_message = get_config_string!(data, guild_id, ConfigKey::GreetMessage);
    let greet_channel = get_config_chan!(ctx, data, guild_id, ConfigKey::GreetChannel);
    let default_role = if add_role {
        get_config_role_option!(ctx, data, guild_id, ConfigKey::GreetDefaultRole)
    } else {
        None
    };

    // Do the default role first if it's configured as this is a "security" feature
    if let Some(default_role) = default_role {
        if !member.roles.contains(&default_role.id) {
            member.add_role(ctx, default_role.id)
                .await
                .context("Adding default role")?;
        }
    }

    // Send the greeting
    expand_greeting_template(&member.user, &mut greet_message);
    Embed::default()
        .content(format!("{member}"))
        .description(greet_message)
        .random_color()
        .send_in_channel(greet_channel.id.into(), ctx)
        .await?;

    Ok(OptionallyConfiguredResult::Ok(()))
}
