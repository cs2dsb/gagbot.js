use std::borrow::Cow;
use std::fmt::Write;

use poise::{
    self,
    serenity_prelude::{Color, Member},
};
use tracing::error;

use crate::{
    Context, Embed, EmbedFlavour, Error, commands::{greet::run_greet, promote::OptionallyConfiguredResult},
};

#[poise::command(prefix_command, slash_command, category = "Testing")]
/// Ping pong
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    Embed::default().description("Pong!").send(&ctx).await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Testing")]
/// Test the bot's embed, optionally providing various customisations
pub async fn test_embed(
    ctx: Context<'_>,

    #[description = "Title"] title: Option<String>,
    #[description = "Description (the main text)"] description: Option<String>,
    #[description = "Normal, Error or Success"] flavour: Option<EmbedFlavour>,
    #[description = "Colour of the embed left border as a hex number (0xEBC634). Overrides flavour colour"]
    color: Option<i32>,
    #[description = "Thumbnail to place in the top right"] thumbnail_url: Option<String>,
) -> Result<(), Error> {
    let mut embed = Embed::default();
    if let Some(color) = color {
        embed.color = Some(Color::from(color));
    }

    embed.flavour = flavour;
    embed.thumbnail_url = thumbnail_url;
    embed.title = title;
    embed.description = description;

    embed.send(&ctx).await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Testing")]
/// Displays a generic success message
pub async fn test_embed_success(ctx: Context<'_>) -> Result<(), Error> {
    Embed::success().send(&ctx).await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Testing")]
/// Displays a generic error message
pub async fn test_embed_error(ctx: Context<'_>) -> Result<(), Error> {
    Embed::error().send(&ctx).await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Testing")]
/// Test the bot's greet functionality - doesn't assign any roles, it only posts the
/// greeting message
pub async fn test_greet_message(
    ctx: Context<'_>,

    #[description = "Member to greet. Defaults to your user if not provided"] 
    member: Option<Member>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");

    let member = member
        .map(|v| Cow::Owned(v))
        .or(ctx.author_member().await.map(|v| v.clone()))
        .ok_or(anyhow::anyhow!("Member not provided and author missing from context"))?;

    let mut embed = Embed::default().title("Greeting").ephemeral(true);

    let mut msg = String::new();
    // 0 = ok, 1 = warn, 2 = error
    let mut err_lvl = 0;

    match run_greet(ctx.data(), &ctx, guild_id.into(), member.into_owned(), false).await {
        Ok(OptionallyConfiguredResult::Ok(_)) => {
            write!(&mut msg, ":white_check_mark: Done")?;
        }
        Ok(OptionallyConfiguredResult::Unconfigured(key)) => {
            err_lvl = err_lvl.max(1);
            write!(&mut msg, ":grey_question: Greet not configured: {}", key)?;
        }
        Err(e) => {
            err_lvl = err_lvl.max(2);
            let err = format!(":x: Greet error: {:?}", e);
            error!("Error running greet: {}", err);
            msg.push_str(&err);
        }
    }
    msg.push('\n');

    embed.flavour = Some(match err_lvl {
        0 => EmbedFlavour::Success,
        1 => EmbedFlavour::Normal,
        _ => EmbedFlavour::Error,
    });

    embed = embed.description(msg);
    embed.send(&ctx).await?;

    Ok(())
}
