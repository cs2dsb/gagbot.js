use poise::{
    self,
    serenity_prelude::{ButtonStyle, ChannelId, Color, User},
};

use crate::{
    Context, Embed, EmbedFlavour, Error, INTERACTION_BUTTON_CUSTOM_ID_DELIMITER,
    INTERACTION_BUTTON_CUSTOM_ID_PREFIX,
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
/// Test the bot's greet functionality
pub async fn test_greet(
    ctx: Context<'_>,

    #[description = "Member to greet"] user: User,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");

    if let Some((_channel_id, embed)) = ctx.data().get_greet(guild_id.into(), &user).await? {
        embed.send(&ctx).await?;
    } else {
        Embed::default()
            .description("Greeting is not configured")
            .send(&ctx)
            .await?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Testing")]
/// Test the bot's interaction roles functionality
pub async fn test_interaction_roles(
    ctx: Context<'_>,

    #[description = "Channel to post in"] channel: ChannelId,
) -> Result<(), Error> {
    // let guild = ctx
    //     .guild()
    //     .expect("missing guild in 'guild_only' command");

    // if let Some((_channel_id, embed)) = ctx.data().get_greet(guild_id.into(),
    // &user).await? {     embed
    //         .send(&ctx)
    //         .await?;
    // } else {
    //     Embed::default()
    //         .description("Greeting is not configured")
    //         .send(&ctx)
    //         .await?;
    // }

    let _reply = channel
        .send_message(&ctx, |m| {
            m.embed(|b| {
                Embed::default()
                    .title("Choose your platform")
                    .description("Select the platform(s) you game on")
                    .create_embed(b)
            })
            .components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|b| {
                        b.custom_id(format!(
                            "{}{}platform{}1099011451596845067",
                            INTERACTION_BUTTON_CUSTOM_ID_PREFIX,
                            INTERACTION_BUTTON_CUSTOM_ID_DELIMITER,
                            INTERACTION_BUTTON_CUSTOM_ID_DELIMITER
                        ))
                        .label("PC")
                        .style(ButtonStyle::Success)
                        .emoji('👍')
                    })
                    .create_button(|b| {
                        b.custom_id(format!(
                            "{}{}platform{}1099011543108169748",
                            INTERACTION_BUTTON_CUSTOM_ID_PREFIX,
                            INTERACTION_BUTTON_CUSTOM_ID_DELIMITER,
                            INTERACTION_BUTTON_CUSTOM_ID_DELIMITER
                        ))
                        .label("Playstation")
                        .style(ButtonStyle::Danger)
                        .emoji('👎')
                    })
                })
            })
        })
        .await?;

    ctx.send(|b| b.content("ok")).await?;

    Ok(())
}
