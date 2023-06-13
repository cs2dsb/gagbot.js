use poise::serenity_prelude::{Cache, CacheHttp,   Http, Member};
use tracing::debug;

use crate::{
    get_config_string, get_config_chan,
    db::queries::config::{ConfigKey},
    BotData,  GuildId,  expand_greeting_template, Embed, get_config_role_option, ErrorContext, Error
};


use super::promote::OptionallyConfiguredResult;

#[derive(Debug, Clone, Copy)]
pub enum GreetBehaviour {
    Greet,
    ApplyRole,
    Both,
}

impl GreetBehaviour {
    fn greet(&self) -> bool {
        match self {
            Self::Greet | Self::Both => true,
            _ => false,
        }
    }

    fn apply_role(&self) -> bool {
        match self {
            Self::ApplyRole | Self::Both => true,
            _ => false,
        }
    }
}

pub async fn run_greet<'a, 'b, T>(
    data: &'a BotData,
    ctx: &'a T,
    guild_id: GuildId,
    mut member: Member,
    behaviour: GreetBehaviour,
) -> Result<OptionallyConfiguredResult<()>, Error>
where
    T: 'a + Clone + CacheHttp + AsRef<Cache> + AsRef<Http>,
{
    // Do the default role first if it's configured as this is a "security" feature
    if behaviour.apply_role() {
        if let Some(default_role) = get_config_role_option!(ctx, data, guild_id, ConfigKey::GreetDefaultRole) {
            if !member.roles.contains(&default_role.id) {
                debug!("Adding GreetDefaultRole to {}", member);
                member.add_role(ctx, default_role.id)
                    .await
                    .context("Adding default role")?;
            } else {
                debug!("Not adding GreetDefaultRole to {}, it already exists", member);
            }
        } else {
            debug!("GreetDefaultRole not configured");
        }
    }

    if !behaviour.greet() {
        // Nothing left to do
        return Ok(OptionallyConfiguredResult::Ok(()));
    }

    // Get all the config we will need
    let mut greet_message = get_config_string!(data, guild_id, ConfigKey::GreetMessage); 
    let greet_channel = get_config_chan!(ctx, data, guild_id, ConfigKey::GreetChannel);

    // Send the greeting
    debug!("Greeting {}", member);
    expand_greeting_template(&member.user, &mut greet_message);
    Embed::default()
        .content(format!("{member}"))
        .description(greet_message)
        .random_color()
        .send_in_channel(greet_channel.id.into(), ctx)
        .await?;

    Ok(OptionallyConfiguredResult::Ok(()))
}
