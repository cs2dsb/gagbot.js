use chrono::Utc;
use futures::StreamExt;
use poise::{
    self,
    serenity_prelude::{
        Cache, CacheHttp, ChannelId, Http, MessageId, MessagesIter, Timestamp, User,
    },
};

use crate::{
    ErrorContext,
    db::queries::{
        config::LogChannel,
        message_log::LogType,
        permissions::{Permission, PermissionCheck},
    },
    with_progress_embed, BotData, Context, PoiseError, commands::promote::OptionallyConfiguredResult,
};

#[poise::command(prefix_command, slash_command, category = "Utils")]
/// Purge messages en masse
pub async fn purge<'a>(
    ctx: Context<'a>,
    #[description = "The ID of the message AFTER which messages will be purged"] after_id: String,
    #[description = "Optionally, a user to filter with - messages by other users will not be deleted"]
    filter_user: Option<User>,
    #[description = "Limit the number deleted (working backwards from the newest message). Defaults to 50"]
    limit: Option<u64>,
) -> Result<(), PoiseError> {
    ctx.defer_ephemeral().await?;
    ctx.require_permission(Permission::MessagePurge).await?;
    let channel_id = ctx.channel_id();
    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");
    // We don't want to delete anything that is created after the command was issued
    let until_timestamp = ctx.created_at();
    let limit = limit.unwrap_or(50);

    async fn work<'a, Ctx>(
        ctx: &'a Ctx,
        (channel_id, after_id, filter_user, until_timestamp, data, mut limit): (
            ChannelId,
            String,
            Option<User>,
            Timestamp,
            &BotData,
            u64,
        ),
        progress_chan: flume::Sender<String>,
    ) -> Result<OptionallyConfiguredResult<()>, PoiseError>
    where
        Ctx: 'a + CacheHttp + AsRef<Http> + AsRef<Cache>,
    {
        let after_id = MessageId::from(
            after_id
                .parse::<u64>()
                .with_context(|| format!("Cannot parse \"{after_id}\" as unsigned int"))?,
        );

        let user_id = filter_user.map(|v| v.id);
        let mut batch = Vec::new();

        async fn delete_batch<'a, Ctx: 'a + CacheHttp + AsRef<Http>>(
            ctx: Ctx,
            data: &BotData,
            channel_id: &ChannelId,
            batch: &mut Vec<MessageId>,
            progress_chan: &flume::Sender<String>,
        ) -> Result<(), PoiseError> {
            if batch.len() > 0 {
                progress_chan
                    .send_async(format!("Deleting {} messages", batch.len()))
                    .await?;
                let now = Utc::now().into();
                // TODO: there should be a transaction around this so an error from discord
                // reverts it
                for id in batch.iter() {
                    data.log_message(
                        id.into(),
                        now,
                        LogType::Purge,
                        None,
                    )
                    .await?;
                }
                channel_id.delete_messages(ctx, batch.drain(..)).await?;
            }
            Ok(())
        }

        progress_chan
            .send_async("Fetching messages".to_string())
            .await?;
        let mut messages = MessagesIter::<Http>::stream(ctx, channel_id).boxed();
        while let Some(r) = messages.next().await {
            let message = r?;
            if message.id <= after_id {
                // They are ordered newest to oldest so once this check is hit, there won't be
                // any more valid messages coming
                break;
            }
            if message.timestamp > until_timestamp {
                // This skips any newer than the command so we need to continue
                continue;
            }
            if let Some(user_id) = user_id.as_ref() {
                if user_id != &message.author.id {
                    continue;
                }
            }

            batch.push(message.id);
            limit -= 1;
            if limit == 0 {
                break;
            }
            if batch.len() == 100 {
                delete_batch(
                    ctx,
                    data,
                    &channel_id,
                    &mut batch,
                    &progress_chan,
                )
                .await?;
            }
        }
        delete_batch(
            ctx,
            data,
            &channel_id,
            &mut batch,
            &progress_chan,
        )
        .await?;
        progress_chan.send_async("Done".to_string()).await?;

        Ok(OptionallyConfiguredResult::Ok(()))
    }

    const PURGE_TITLE: &str = "Purge";
    let message = ctx.send(|b| b
        .ephemeral(true)
        .content("Purging...")
    ).await?;

    match with_progress_embed(
        ctx.data(),
        &ctx,
        guild_id.into(),
        LogChannel::EditsAndDeletes,
        PURGE_TITLE,
        work,
        (
            channel_id,
            after_id,
            filter_user,
            until_timestamp,
            ctx.data(),
            limit,
        ),
    )
    .await
    {
        Ok(_) => {
            message.edit(ctx, |b| b.content("Done")).await?;
            message.delete(ctx).await
        },
        Err(e) => {
            message
                .edit(ctx, |b| b.content(format!("Error purging: {:?}", e)))
                .await
        }
    }?;

    Ok(())
}
