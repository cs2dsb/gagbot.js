use poise::{
    self,
    serenity_prelude::{ChannelId, UserId},
};

use crate::{Context, Embed, Error};

#[poise::command(prefix_command, slash_command, guild_only, category = "Stats")]
/// Print the total number of messages we've counted from a user
pub async fn message_count(
    ctx: Context<'_>,
    #[description = "The user you want the message count for"] user_id: UserId,
    #[description = "(Optionally) The channel you want a message count for"] channel_id: Option<
        ChannelId,
    >,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");

    let r = ctx
        .data()
        .message_count(
            guild_id.into(),
            user_id.into(),
            channel_id.map(|v| v.into()),
        )
        .await;
    match r {
        Ok(count) => {
            Embed::success()
                .title("We've counted")
                .description(format!(
                    "{} messages from <@{}> in <#{}>",
                    count,
                    user_id,
                    channel_id
                        .map(|v| v.to_string())
                        .unwrap_or("all channels".to_string())
                ))
                .send(ctx)
                .await?
        }
        Err(e) => Embed::error().description(e.to_string()).send(ctx).await?,
    };

    Ok(())
}
