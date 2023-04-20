// TODO:
//
// Events:
// on_voiceStateUpdate
// Log when a joins, leaves, or moves to a different voice channel

// on_messageReactionAdd
// Add role corresponding to emoji
// on_messageReactionRemove
// Remove role corresponding to emoji
//
//
// admin
// "prune", "Kick inactive users", "gagbot:admin:prune"
// "purge", "Bulk delete recent messages", "gagbot:admin:purge"
//
// core
// "permcheck","List all permissions for a role, under a certain node.",
// "gagbot:permission:list", { "roleID": roleID, "node": str }
// "permclear","Clear all permission nodes for a role","gagbot:permission:set",
// { "roleID": roleID }); "permlist", "List all permissions for a given
// role.","gagbot:permission:list", { "roleID": roleID }); "permset", "Set a
// permission node for a role", "gagbot:permission:set",{ "roleID": roleID,
// "node": str, "allow": bool }); "permunset", "Unset a permission node for a
// role","gagbot:permission:set", { "roleID": roleID, "node": str }); "am", "Add
// a member.", "gagbot:greet:addmember", [user] "greet", "Send a greeting to the
// given user.", "gagbot:greet:send", [user] "greetchannel", "Set the greeting
// channel.", "gagbot:greet:set", [channel] "greetdelete", "Stop greeting
// users.", "gagbot:greet:delete" "greetmessage", "Set the greeting message.",
// "gagbot:greet:set", [str] "greetrole", "Set the member role.",
// "gagbot:greet:set", [role] "greetwelcomechannel", "Set the welcome channel.",
// "gagbot:greet:set", [channel] "greetwelcomemessage", "Set the greeting
// welcome message (for after !am).", "gagbot:greet:set", [str] "promote",
// "Promote eligible members: New to junior members when they have > 2 roles
// set. Junior to full when they've been active on the server for > 3 days.
// Optionally pass a user to promote them without any checks",
// "gagbot:greet:promotemembers", [optional(user)] "promoteroles", "Set the
// promotion member roles. Example: `!promoteroles @Junior Member @Member`",
// "gagbot:promoteroles:set", [role, role] "promoterules", "Set the promotion
// rules. \nUsage `#new-member-channel-to-scan #junior-member-channel-to-scan
// min-new-member-messages min-junior-member-messages
// min-junior-member-age-in-days`. \nExample: `!promoterules #introduce-yourself
// #general 1 10 3`", "gagbot:promoterrules:set", [channel, channel, num, num,
// num] "log", "Set which events to log in which channel.",
// "gagbot:logging:channel", { 'cmd': choice(i('set'), i('list'), i('check'),
// i('delete')), 'channel': optional(channel), 'types':
// optional(some(choice(i('message'), i('voice'), i('member'), i('error')))) }
// 'rrbind', 'Bind a roleset to a message.', 'gagbot:reactionroles:bind', {
// 'set': str, 'channel': channel, 'message': id } 'rrset', 'Modify rolesets.',
// 'gagbot:reactionroles:set', { 'cmd': choice(i('add'), i('delete'),
// i('clear'), i('update'), i('list'), i('togglex')),'set': str, 'react':
// optional(choice(emoji, str)),'role': optional(role)} 'rrunbind', 'Unbind a
// roleset from it\'s bound message.', 'gagbot:reactionroles:bind' 'rrupdate',
// 'Ensure all the correct roles are on the react menu for a given roleset.',
// 'gagbot:reactionroles:bind', { 'set': str } "tafk","Toggle whether or not to
// move inactive voice users to the AFK
// channel.","gagbot:voice:toggleafk",{channel: optional(id)}
//

use clap::Parser;
use dotenv::dotenv;
use futures::future::join;
use gagbot_rs::{config::LogChannel, *};
use poise::{
    self,
    serenity_prelude::{self as serenity, CacheHttp, GatewayIntents, Timestamp},
    FrameworkContext, FrameworkError,
};
use tracing::*;

#[derive(Debug, Parser)]
#[clap(name = "gagbot.rs")]
struct Cli {
    #[clap(long, env)]
    discord_token: String,
    #[clap(long, env, default_value = "gagbot.sqlite")]
    sqlite_connection_string: String,
    #[clap(long, env, default_value = "64")]
    database_command_channel_bound: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;
    configure_tracing();

    let args = Cli::parse();
    debug!("Parsed args: {:#?}", args);

    // Open the DB before launching the task so we can fail before trying to connect
    // to discord
    let mut sqlite_con = open_database(&args.sqlite_connection_string, true)?;
    let (sender, receiver) = flume::bounded::<DbCommand>(args.database_command_channel_bound);

    let db_task_handle = tokio::spawn(async move {
        debug!("DB TASK: started");
        loop {
            tokio::select! {
                r = receiver.recv_async() => match r {
                    // The only error it returns is Disconnected (which we use to shut down)
                    Err(_) => break,
                    Ok(cmd) => {
                        trace!("DB TASK: got command: {:?}", cmd);
                        match cmd {
                            DbCommand::GetConfigString { guild_id, key, respond_to } => {
                                respond_to.send(config::get(&sqlite_con, guild_id, key))
                                    .map_err(|_| anyhow::anyhow!("GetConfigString respond_to oneshot closed"))?;
                            },
                            DbCommand::GetConfigI64 { guild_id, key, respond_to } => {
                                respond_to.send(config::get(&sqlite_con, guild_id, key))
                                    .map_err(|_| anyhow::anyhow!("GetConfigI64 respond_to oneshot closed"))?;
                            },
                            DbCommand::SetConfigString { guild_id, key, value, timestamp, respond_to } => {
                                respond_to.send(config::update(&sqlite_con, guild_id, key, &value, timestamp))
                                    .map_err(|_| anyhow::anyhow!("SetConfigString respond_to oneshot closed"))?;
                            },
                            DbCommand::DeleteConfig { guild_id, key, timestamp, respond_to } => {
                                respond_to.send(config::delete(&sqlite_con, guild_id, key, timestamp))
                                    .map_err(|_| anyhow::anyhow!("DeleteConfig respond_to oneshot closed"))?;
                            },
                            DbCommand::GetLogChannel { guild_id, purpose, respond_to } => {
                                respond_to.send(config::get_log_channel(&sqlite_con, guild_id, &purpose))
                                    .map_err(|_| anyhow::anyhow!("GetLogChannel respond_to oneshot closed"))?;
                            },
                            DbCommand::GetMessageCount { guild_id, user_id, channel_id, respond_to } => {
                                respond_to.send(message_count::get(&sqlite_con, guild_id, user_id, channel_id))
                                    .map_err(|_| anyhow::anyhow!("GetMessageCount respond_to oneshot closed"))?;
                            },
                            DbCommand::IncrementMessageCount { guild_id, user_id, channel_id, respond_to } => {
                                respond_to.send(message_count::increment(&sqlite_con, guild_id, user_id, channel_id))
                                    .map_err(|_| anyhow::anyhow!("IncrementMessageCount respond_to oneshot closed"))?;
                            },
                            DbCommand::GetGreet { guild_id, respond_to } => {
                                respond_to.send(config::get_greet(&sqlite_con, guild_id))
                                    .map_err(|_| anyhow::anyhow!("GetGreet respond_to oneshot closed"))?;
                            },
                            DbCommand::GetMemberPermissions { guild_id, sorted_roles, respond_to } => {
                                respond_to.send(permissions::get(&sqlite_con, guild_id, sorted_roles))
                                    .map_err(|_| anyhow::anyhow!("GetUserPermissions respond_to oneshot closed"))?;
                            },
                            DbCommand::GrantPermission { guild_id, role_id, permission, respond_to, timestamp } => {
                                respond_to.send(permissions::grant(&sqlite_con, guild_id, role_id, permission, timestamp))
                                    .map_err(|_| anyhow::anyhow!("GrantPermission respond_to oneshot closed"))?;
                            },
                            DbCommand::RevokePermission { guild_id, role_id, permission, respond_to, timestamp } => {
                                respond_to.send(permissions::grant(&sqlite_con, guild_id, role_id, permission, timestamp))
                                    .map_err(|_| anyhow::anyhow!("RevokePermission respond_to oneshot closed"))?;
                            },
                            DbCommand::PurgePermissions { guild_id, respond_to, timestamp } => {
                                respond_to.send(permissions::purge(&mut sqlite_con, guild_id, timestamp))
                                    .map_err(|_| anyhow::anyhow!("PurgePermissions respond_to oneshot closed"))?;
                            },
                        }
                    },
                },
                // Can be used to test poise graceful shutdown
                // _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                //     Err(anyhow::anyhow!("DB fake error"))?;
                // }
            }
        }
        debug!("DB TAST: exiting");

        close_database(sqlite_con)?;

        Ok::<_, anyhow::Error>(())
    });

    let options = poise::FrameworkOptions {
        commands: commands::commands(),
        on_error: |err| Box::pin(on_error(err)),
        event_handler: |ctx, event, framework, data| {
            Box::pin(event_handler(ctx, event, framework, data))
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .client_settings(|b| b.cache_settings(|s| s.max_messages(CACHE_MAX_MESSAGES)))
        .options(options)
        .token(args.discord_token)
        // TODO: are all needed?
        .intents(
            GatewayIntents::GUILDS
                | GatewayIntents::GUILD_MEMBERS
                | GatewayIntents::GUILD_BANS
                | GatewayIntents::GUILD_EMOJIS_AND_STICKERS
                | GatewayIntents::GUILD_INTEGRATIONS
                | GatewayIntents::GUILD_WEBHOOKS
                | GatewayIntents::GUILD_INVITES
                | GatewayIntents::GUILD_VOICE_STATES
                | GatewayIntents::GUILD_PRESENCES
                | GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::GUILD_MESSAGE_REACTIONS
                | GatewayIntents::GUILD_MESSAGE_TYPING
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::DIRECT_MESSAGE_REACTIONS
                | GatewayIntents::DIRECT_MESSAGE_TYPING
                | GatewayIntents::MESSAGE_CONTENT
                | GatewayIntents::GUILD_SCHEDULED_EVENTS
                | GatewayIntents::AUTO_MODERATION_CONFIGURATION
                | GatewayIntents::AUTO_MODERATION_EXECUTION,
        )
        .setup(|_ctx, _ready, _framework| {
            debug!("Discord connected");
            Box::pin(async move { Ok(BotData::new(sender)) })
        })
        .build()
        .await?;

    let shard_manager_handle = framework.client().shard_manager.clone();
    let (framework_r, db_r) = join(
        // Don't need to do anything special in this case as the dropped sender will cause the db
        // task to stop
        framework.start(),
        // In this case however, if the db exits first the framework needs to be shut down
        async move {
            let r = db_task_handle.await;
            shard_manager_handle.lock().await.shutdown_all().await;
            r
        },
    )
    .await;

    // First ? is for join result, 2nd is for the actual task result
    db_r??;
    framework_r?;

    Ok(())
}

async fn on_error(error: FrameworkError<'_, BotData, Error>) {
    
    let msg = match &error {
        FrameworkError::ArgumentParse { error, ..} => {
            Some(error.to_string())
        },
        FrameworkError::Command { error, .. } => {
            // TODO: Probably do something more fancy with user facing errors and the like?
            Some(error.to_string())
        },
        
        e => {
            warn!("UNHANDLED error from poise: {:?}", e);
            None
        },
    };

    if let (Some(msg), Some(ctx)) = (msg, error.ctx().as_ref()) {
        if let Err(e) = Embed::error()
            .description(msg)
            .send(ctx).await
        {
            error!("Error from ctx.send in error handler: {:?}", e);
        }
    }
}

async fn event_handler<'a>(
    ctx: &serenity::Context,
    event: &'a poise::Event<'a>,
    framework: FrameworkContext<'a, BotData, Error>,
    data: &'a BotData,
) -> Result<(), Error> {
    use poise::Event::*;
    debug!("got event: {}", event.name());
    trace!("EVENT VALUE: {:#?}", event);
    match event {
        GuildCreate {
            guild, ..
        } => {
            poise::builtins::register_in_guild(ctx, &framework.options().commands, guild.id)
                .await?;

            // TODO: currently this kind of ? will crash the bot. Is this what we want to
            // happen on DB errors?
            let channel_id = data
                .general_log_channel(guild.id.into())
                .await?
                .or(guild.system_channel_id.map(|v| v.into()))
                .or(guild
                    .default_channel_guaranteed()
                    .map(|c| c.id)
                    .map(|v| v.into()));

            if let Some(chan) = channel_id {
                Embed::default()
                    .title("I'm here")
                    .footer(format!("{}", Timestamp::now().to_rfc2822()))
                    .send_in_channel(chan, &ctx.http)
                    .await?;
            } else {
                warn!("Failed to get log, system or default channels to log to");
            }
        }
        Message {
            new_message,
        } => {
            // We only want in guild messages
            if let Some(guild_id) = new_message.guild_id {
                let user_id = new_message.author.id;
                let channel_id = new_message.channel_id;

                data.increment_message_count(guild_id.into(), user_id.into(), channel_id.into())
                    .await?;
            }
        }
        MessageUpdate {
            old_if_available,
            new,
            event,
        } => {
            if let (Some(guild_id), Some(old), Some(new)) = (event.guild_id, old_if_available, new)
            {
                let user = &old.author;
                // TODO: handle images and videos? They currently don't show
                if !user.bot && old.content != new.content {
                    if let Some(channel_id) = data
                        .log_channel(guild_id.into(), vec![LogChannel::EditsAndDeletes])
                        .await?
                    {
                        Embed::default()
                            .title(format!("{} edited their message", user.tag()))
                            .description(format!(
                                "**Before**\n```\n{}\n```\n**After**\n```\n{}\n```",
                                old.content, new.content
                            ))
                            .flavour(EmbedFlavour::LogEdit)
                            .send_in_channel(channel_id, &ctx.http)
                            .await?;
                    }
                }
            }
        }
        MessageDelete {
            channel_id,
            deleted_message_id,
            guild_id,
        } => {
            if let (Some(guild_id), Some(cache)) = (guild_id, ctx.cache()) {
                if let Some(message) = cache.message(channel_id, deleted_message_id) {
                    let user = &message.author;
                    if !user.bot {
                        if let Some(channel_id) = data
                            .log_channel((*guild_id).into(), vec![LogChannel::EditsAndDeletes])
                            .await?
                        {
                            Embed::default()
                                .title(format!("Message from {} was deleted", user.tag()))
                                .description(format!("```\n{}\n```", message.content,))
                                .flavour(EmbedFlavour::LogDelete)
                                .send_in_channel(channel_id, &ctx.http)
                                .await?;
                        }
                    }
                } else {
                    warn!(
                        "Failed to look up deleted_message_id ({}/{}) from cache for guild {}",
                        channel_id, deleted_message_id, guild_id
                    );
                }
            }
        }
        GuildMemberAddition {
            new_member,
        } => {
            let guild_id = new_member.guild_id;     
            let user = &new_member.user;       

            if let Some((channel_id, embed)) = data.get_greet(guild_id.into(), user).await? {
                embed
                    .send_in_channel(channel_id, &ctx.http)
                    .await?;
            }

            if let Some(channel_id) = data.log_channel(guild_id.into(), vec![LogChannel::JoiningAndLeaving]).await? {
                Embed::join()
                    .description(format!(
                        "`{}` joined the server.",
                        user.tag()))
                    .send_in_channel(channel_id, &ctx.http)
                    .await?;
            }
        }
        GuildMemberRemoval { guild_id, user, .. } => {
            if let Some(channel_id) = data.log_channel((*guild_id).into(), vec![LogChannel::JoiningAndLeaving]).await? {
                // TODO: check audit log for kick status
                Embed::leave()
                    .description(format!(
                        "`{}` left the server.",
                        user.tag()))
                    .send_in_channel(channel_id, &ctx.http)
                    .await?;
            } 
        }
        _ => {}
    }

    Ok(())
}
