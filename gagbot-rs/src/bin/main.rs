// TODO:
//
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

use chrono::{Utc, DateTime};
use clap::Parser;
use dotenv::dotenv;
use futures::future::join;
use gagbot_rs::{config::LogChannel, *, message_log::{LogType, MessageLog}};
use humansize::{make_format, BINARY};
use poise::{self, FrameworkContext, FrameworkError, serenity_prelude::{self as serenity, ActionRowComponent, CacheHttp, ComponentType, GatewayIntents, Interaction, Timestamp, Message, Context, Guild, MessageUpdateEvent, VoiceState}};
use tokio::time;
use tracing::*;
use std::{fmt::Write, time::Duration, num::ParseIntError};


fn frequency_seconds_valid_range(s: &str) -> Result<u64, String> {
    let v = s.parse()
        .map_err(|e: ParseIntError| e.to_string())?;

    if v < 60 {
        Err(format!("Running more often than once per minute ({} seconds) is not recommended", v))?;
    }
    Ok(v)
}

#[derive(Debug, Parser)]
#[clap(name = "gagbot.rs")]
struct Cli {
    #[clap(long, env)]
    discord_token: String,
    #[clap(long, env, default_value = "gagbot.sqlite")]
    sqlite_connection_string: String,
    #[clap(long, env, default_value = "64")]
    database_command_channel_bound: usize,
    #[clap(long, env, default_value = "3600", value_parser = frequency_seconds_valid_range)]
    background_task_frequency_seconds: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;
    configure_tracing();

    let args = Cli::parse();
    let background_task_frequency = Duration::from_secs(args.background_task_frequency_seconds);
    debug!("Parsed args: {:#?}", args);

    // Open the DB before launching the task so we can fail before trying to connect
    // to discord
    let mut sqlite_con = open_database(&args.sqlite_connection_string, true)?;
    let db_file_path = sqlite_con.path().map(|p| p.to_owned());
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
                            DbCommand::GetConfigU64 { guild_id, key, respond_to } => {
                                respond_to.send(config::get(&sqlite_con, guild_id, key))
                                    .map_err(|_| anyhow::anyhow!("GetConfigU64 respond_to oneshot closed"))?;
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
                            DbCommand::UpdateInteractionRoleSet { guild_id, name, description, channel_id, message_id, exclusive, timestamp, respond_to } => {
                                respond_to.send(interaction_roles::update(&sqlite_con, guild_id, name, description, channel_id, message_id, exclusive, timestamp))
                                    .map_err(|_| anyhow::anyhow!("UpdateInteractionRoleSet respond_to oneshot closed"))?;
                            },
                            DbCommand::GetInteractionRole { guild_id, name, respond_to } => {
                                respond_to.send(interaction_roles::get(&sqlite_con, guild_id, name ))
                                    .map_err(|_| anyhow::anyhow!("GetInteractionRoleSet respond_to oneshot closed"))?;
                            },
                            DbCommand::UpdateInteractionRoleChoice { guild_id, set_name, choice, emoji, role_id, timestamp, respond_to } => {
                                respond_to.send(interaction_roles::update_choice(&sqlite_con, guild_id, set_name, choice, emoji, role_id, timestamp ))
                                    .map_err(|_| anyhow::anyhow!("UpdateInteractionRoleChoice respond_to oneshot closed"))?;
                            },
                            DbCommand::LogMessage { guild_id, user_id, channel_id, message_id, timestamp, type_, message, respond_to } => {
                                respond_to.send(message_log::log(&sqlite_con, guild_id, user_id, channel_id, message_id, timestamp, type_, message))
                                    .map_err(|_| anyhow::anyhow!("LogMessage respond_to oneshot closed"))?;
                            },
                            DbCommand::GetLogMessages { guild_id, channel_id, message_id, respond_to } => {
                                respond_to.send(message_log::get(&sqlite_con, guild_id, channel_id, message_id))
                                    .map_err(|_| anyhow::anyhow!("GetLogMessages respond_to oneshot closed"))?;
                            },                            
                            DbCommand::GetUserFromLogMessages{ guild_id, channel_id, message_id, respond_to } => {
                                respond_to.send(message_log::get_user(&sqlite_con, guild_id, channel_id, message_id))
                                    .map_err(|_| anyhow::anyhow!("GetUserFromLogMessages respond_to oneshot closed"))?;
                            },                         
                            DbCommand::GetTableBytes { respond_to } => {
                                respond_to.send(get_table_bytes(&sqlite_con))
                                    .map_err(|_| anyhow::anyhow!("GetTableBytes respond_to oneshot closed"))?;
                            }                            
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
        .setup(move |_ctx, _ready, _framework| {
            debug!("Discord connected");
            Box::pin(async move { Ok(BotData::new(sender, db_file_path, background_task_frequency)) })
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

// TODO: make sure result is handled
async fn background_tasks(data: BotData, ctx: Context, guild: Guild) -> anyhow::Result<()> {
    // Prime the cache. It will be kept up to date after this by events
    // TODO: This won't fetch more than 1000. If there are that many members
    //       we should probably not rely on the cache anyway which would mean
    //       switching the promote functionality to event based
    let _ = guild.members(&ctx, None, None).await?;

    let formatter = make_format(BINARY);
    let mut tick_interval = time::interval(data.background_task_frequency);
    let tick_duration = chrono::Duration::from_std(data.background_task_frequency)?;
    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                let next_run: DateTime<Utc> = Utc::now() + tick_duration;

                debug!("Running background tasks");

                let log_channel_id = match data
                    .general_log_channel(guild.id.into())
                    .await
                {
                    Err(e) => {
                        error!("Error getting general log channel in background_tasks for guild: {} ({}): {:?}", guild.name, guild.id, e);
                        continue
                    },
                    Ok(None) => continue,
                    Ok(Some(v)) => v,
                };

                let mut embed = Embed::default()
                    .title("Ran background tasks");

                let mut msg = String::new();
                // 0 = ok, 1 = warn, 2 = error
                let mut err_lvl = 0;

                match data.db_available_space() {
                    Ok(bytes) if bytes > DISK_SPACE_WARNING_LEVEL => {
                        write!(&mut msg, ":white_check_mark: Disk space ok: {}", formatter(bytes))?;
                    },
                    Ok(bytes) => {
                        err_lvl = err_lvl.max(1);
                        write!(&mut msg, ":x: Disk space low: {}", formatter(bytes))?;
                    },
                    Err(e) => {
                        err_lvl = err_lvl.max(2);
                        let err = format!(":x: Disk space error: {:?}", e);
                        error!("Error checking disk space: {}", err);
                        msg.push_str(&err);
                    }
                }
                msg.push('\n');

                match data.db_table_sizes().await {
                    Ok(tables) => {
                        let total = tables.into_iter().fold(0, |a, t| a + t.1);
                        write!(&mut msg, ":white_check_mark: DB table size total: {}", formatter(total))?;
                    },
                    Err(e) => {
                        err_lvl = err_lvl.max(2);
                        let err = format!(":x: DB table size error: {:?}", e);
                        error!("Error getting DB table sizes: {}", err);
                        msg.push_str(&err);
                    }
                }
                msg.push('\n');

                match run_promote(&data, &ctx, guild.id.into(), None).await {
                    Ok(PromoteResult::Ok(promotions)) => {
                        write!(&mut msg, ":white_check_mark: {}", promotions)?;
                    },
                    Ok(PromoteResult::Unconfigured(key)) => {
                        err_lvl = err_lvl.max(1);
                        write!(&mut msg, ":grey_question: Promote not configured: {}", key)?;
                    },
                    Err(e) => {
                        err_lvl = err_lvl.max(2);
                        let err = format!(":x: Promote error: {:?}", e);
                        error!("Error running promotions: {}", err);
                        msg.push_str(&err);
                    }
                }
                msg.push('\n');

                write!(&mut msg, "\nNext run: <t:{0}:R> (<t:{0}>)", next_run.timestamp())?;

                embed.flavour = Some(match err_lvl {
                    0 => EmbedFlavour::Success,
                    1 => EmbedFlavour::Normal,
                    _ => EmbedFlavour::Error,
                });

                embed = embed.description(msg);

                if let Err(e) = embed
                    .send_in_channel(log_channel_id, &ctx.http)
                    .await 
                {
                    error!("Error posting in general log channel in background_tasks for guild: {} ({}): {:?}", guild.name, guild.id, e);
                }
            },
        }
    }
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
        GuildCreate { guild, .. } => handle_guild_create(ctx, data, framework, guild).await?,
        Message { new_message } => handle_message_create(data, new_message).await?,
        MessageUpdate { old_if_available, new, event } => handle_message_update(data, ctx, event, old_if_available, new).await?,
        MessageDelete { channel_id, deleted_message_id, guild_id } => handle_message_delete(data, ctx, guild_id, channel_id, deleted_message_id).await?,
        GuildMemberAddition { new_member } => handle_guild_member_add(data, ctx, new_member).await?,
        GuildMemberRemoval { guild_id, user, .. } => handle_guild_member_remove(data, ctx, guild_id, user).await?,
        InteractionCreate { interaction } => handle_message_component_interaction(ctx, data, interaction).await?,
        VoiceStateUpdate { old, new } => handle_voice_state_update(ctx, data, old, new).await?,
        _ => {}
    }

    Ok(())
}

async fn handle_voice_state_update(ctx: &Context, data: &BotData, old: &Option<VoiceState>, new: &VoiceState) -> anyhow::Result<()> {
    // Filter out bots
    if new.member.as_ref().map(|v| v.user.bot).unwrap_or(false) {
        return Ok(());
    }

    // Filter out voice DMs (if that's even possible?)
    let guild_id = match new.guild_id {
        Some(guild_id) => guild_id,
        None => return Ok(()),
    };

    // Exit now if logging isn't configured
    let log_channel_id = match data.log_channel(guild_id.into(), vec![LogChannel::VoiceActivity]).await? {
        Some(log_channel_id) => log_channel_id,
        None => return Ok(()),
    };

    let old_channel = old
        .as_ref()
        .map(|v| v.channel_id)
        .flatten();
    let new_channel = new.channel_id;
    let user_id = new.user_id.0;
    let timestamp = new.request_to_speak_timestamp.unwrap_or(Utc::now().into()).timestamp();

    let msg = match (old_channel, new_channel) {
        (Some(o), Some(n)) => format!("<t:{timestamp}>: <@{user_id}> moved from <#{o}> to <#{n}>"),
        (Some(o), None) => format!("<t:{timestamp}>: <@{user_id}> disconnected <#{o}>"),
        (None, Some(n)) => format!("<t:{timestamp}>: <@{user_id}> joined <#{n}>"),
        // I feel like this should be impossible?
        (None, None) => return Ok(()),        
    };

    Embed::default()
        .flavour(EmbedFlavour::LogVoice)
        .title("VC Update")
        .description(msg)
        .send_in_channel(log_channel_id, &ctx.http)
        .await?;
    
    Ok(())
}

async fn handle_guild_create<'a>(ctx: &Context, data: &BotData, framework: FrameworkContext<'a, BotData, Error>, guild: &Guild) -> anyhow::Result<()> {
    poise::builtins::register_in_guild(ctx, &framework.options().commands, guild.id)
        .await?;
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
            .description(format!("<t:{}>", Utc::now().timestamp()))
            .send_in_channel(chan, &ctx.http)
            .await?;
    } else {
        warn!("Failed to get log, system or default channels to log to");
    }
    tokio::spawn(background_tasks(
        data.clone(),      
        ctx.clone(),
        guild.clone(),
    ));
    Ok(())
}

async fn handle_message_create(data: &BotData, new_message: &Message) -> anyhow::Result<()> {
    Ok(if let Some(guild_id) = new_message.guild_id {
        let user_id = new_message.author.id;
    
        let channel_id = new_message.channel_id;
        let message_id = new_message.id;

        data.increment_message_count(guild_id.into(), user_id.into(), channel_id.into())
            .await?;                

        data.log_message(
            guild_id.into(), 
            Some(user_id.into()), 
            channel_id.into(),
            message_id.into(),
            new_message.timestamp,
            LogType::Create,
            Some(new_message.clone()),
        ).await?;
    })
}

async fn handle_guild_member_remove(data: &BotData, ctx: &Context, guild_id: &serenity::GuildId, user: &serenity::User) -> anyhow::Result<()> {
    Ok(if let Some(channel_id) = data.log_channel((*guild_id).into(), vec![LogChannel::JoiningAndLeaving]).await? {
        // TODO: check audit log for kick status
        Embed::leave()
            .description(format!(
                "`{}` left the server.",
                user.tag()))
            .send_in_channel(channel_id, &ctx.http)
            .await?;
    })
}

async fn handle_guild_member_add(data: &BotData, ctx: &Context, new_member: &serenity::Member) -> anyhow::Result<()> {
    let guild_id = new_member.guild_id;
    let user = &new_member.user;
    if let Some((channel_id, embed)) = data.get_greet(guild_id.into(), user).await? {
        embed
            .send_in_channel(channel_id, &ctx.http)
            .await?;
    }
    Ok(if let Some(channel_id) = data.log_channel(guild_id.into(), vec![LogChannel::JoiningAndLeaving]).await? {
        Embed::join()
            .description(format!(
                "`{}` joined the server.",
                user.tag()))
            .send_in_channel(channel_id, &ctx.http)
            .await?;
    })
}

async fn handle_message_delete(data: &BotData, ctx: &Context, guild_id: &Option<serenity::GuildId>, channel_id: &serenity::ChannelId, deleted_message_id: &serenity::MessageId) -> anyhow::Result<()> {
    Ok(if let Some(guild_id) = guild_id {                
        // Log to the log channel only if it is configured
        if let Some(log_channel_id) = data
            .log_channel((*guild_id).into(), vec![LogChannel::EditsAndDeletes])
            .await?
        {
            let mut is_bot = false;
            let mut user_id = None;
            let mut msg = format!("**Message {deleted_message_id} in <#{channel_id}> deleted**\n");
            let mut cache_hit = false;

            // This attempts to get it from the cache
            if let Some(cache) = ctx.cache() {
                if let Some(message) = cache.message(channel_id, deleted_message_id) {
                    let content = message_to_string(&message)?
                        .unwrap_or(" ".to_string())
                        .replace("\n", "\n> ");
                    is_bot = message.author.bot;
                    user_id = Some(message.author.id);
                    let user_id = message.author.id.0;
                    let timestamp = message.timestamp.timestamp();
                    if !is_bot {
                        write!(&mut msg, "*Message from cache* (<t:{timestamp}> - <@{user_id}>):\n> {}\n\n", content)?;
                    }
                    cache_hit = true;
                } else {
                    warn!(
                        "Failed to look up deleted_message_id ({}/{}) from cache for guild {}",
                        channel_id, deleted_message_id, guild_id
                    );
                }
            }

            if !cache_hit {
                write!(&mut msg, "*Deleted message was not in the cache.*\n")?;
            }
        
            // Log to the DB, we always do this regardless of config
            data.log_message(
                guild_id.into(), 
                // TODO: Audit log 
                None, 
                channel_id.into(),
                deleted_message_id.into(),
                // Seems like serenity should provide the timestamp of the event from discord but it doesn't seem to
                Timestamp::now(),
                LogType::Delete,
                None,
            ).await?;

            // Attempt to get the audit log entry for the delete
            let mut audit_entry = None;
            if let Some(guild) = ctx.cache.guild(guild_id) {
                let audit_log = match guild.audit_logs(
                    ctx, 
                    // TODO: Magic number from https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object-audit-log-events
                    Some(72), 
                    None, 
                    None, 
                    None,
                ).await {
                    Err(e) => {
                        error!("Error getting audit logs: {:?}", e);
                        None
                    },
                    Ok(logs) => Some(logs),
                };

                if let Some(audit_log) = audit_log {     
                    if audit_log.entries.len() > 0 {
                        // We need a user_id as part of the check
                        if user_id.is_none() {
                            match data.lookup_user_from_message(
                                guild_id.into(), 
                                channel_id.into(), 
                                deleted_message_id.into(),
                            ).await {
                                Ok(user_id_) => user_id = user_id_.map(|v| *v),
                                Err(e) => error!("Error looking up user_id from message log: {:?}", e),
                            }
                        }

                        if let Some(user_id) = user_id {    
                            audit_entry = audit_log
                                .entries
                                .into_iter()
                                .find(|v| {
                                    if let (Some(target_id), Some(options)) = (v.target_id, &v.options) {
                                        if target_id == user_id.0 &&
                                            options.channel_id.as_ref() == Some(channel_id)
                                        {
                                            return true
                                        }
                                    }
                                    false
                                });
                        } else {
                            warn!("Couldn't get a user_id for a deleted message");
                        }
                    }                       
                }
            } else {
                error!("Failed to get Guild instance from ctx.cache");
            }
        

            if !is_bot {
                let log = log_message_history(data, guild_id.into(), channel_id.into(), deleted_message_id.into(), &mut msg).await?;

                if !log
                    .iter()
                    .filter_map(|v| v.message.as_ref())
                    .any(|v| v.author.bot)
                {
                    if let Some(audit_entry) = audit_entry {
                        let deleter = audit_entry.user_id.0;
                        let timestamp = audit_entry.id.created_at().timestamp();
                        write!(&mut msg, "\n*Audit log indicates message was likely deleted by <@{deleter}> at <t:{timestamp}>*")?;
                    } else {
                        write!(&mut msg, "\n*Nothing in the audit log matches so likely a self-delete*")?;
                    }

                    Embed::default()
                        .flavour(EmbedFlavour::LogDelete)
                        .description(msg)
                        .send_in_channel(log_channel_id, &ctx.http)
                        .await?;
                }
            }
        }
    })
}

async fn handle_message_update(data: &BotData, ctx: &Context, event: &MessageUpdateEvent, old_if_available: &Option<Message>, new: &Option<Message>) -> anyhow::Result<()> {
    Ok(if let (Some(guild_id), Some(old), Some(new)) = (event.guild_id, old_if_available, new)
    {
        let user = &old.author;
        if !user.bot && old.content != new.content {
            if let Some(channel_id) = data
                .log_channel(guild_id.into(), vec![LogChannel::EditsAndDeletes])
                .await?
            {
                let channel_id_n = channel_id.0;
                let message_id = new.id.0;
                let user_id = user.id.0;
                let before = &old.content;
                let after = &new.content;
                let before_timestamp = old.timestamp.timestamp();
                let after_timestamp = new.timestamp.timestamp();
                Embed::default()
                    .description(format!("**Message {message_id} in <#{channel_id_n}> edited by <@{user_id}>**\n<t:{before_timestamp}> before:\n> {before}\n<t:{after_timestamp}> after:\n> {after}"))
                    .flavour(EmbedFlavour::LogEdit)
                    .send_in_channel(channel_id, &ctx.http)
                    .await?;
            }
        }

        data.log_message(
            guild_id.into(), 
            Some(new.author.id.into()), 
            new.channel_id.into(),
            new.id.into(),
            event.edited_timestamp.unwrap_or(Timestamp::now()),
            LogType::Edit,
            Some(new.clone()),
        ).await?;
    })
}

fn message_to_string(message: &Message) -> anyhow::Result<Option<String>> {
    let mut content = message.content.clone();

    for e in message.embeds.iter() {
        if content.len() > 0 {
            content.push('\n');
        }
        content.push_str("**Embed**\n");
        if let Some(v) = e.title.as_ref() {
            write!(&mut content, "*{v}*\n")?;
        }
        if let Some(v) = e.description.as_ref() {
            write!(&mut content, "{v}")?;
        }
    }

    for a in message.attachments.iter() {
        if content.len() > 0 {
            content.push('\n');
        }
        content.push_str("**Attachment**\n");
        write!(&mut content, "*{}*\n", a.filename)?;        
        if let Some(v) = a.content_type.as_ref() {
            write!(&mut content, "{v}\n")?;
        }        
        write!(&mut content, "{}", a.url)?;        
    }    

    let content = if content.len() > 0 {        
        Some(content)
    } else {
        debug!("message_to_string empty message: {:#?}", message);
        None
    };

    Ok(content)
}

/// Logs the message history to the provided writer. Also returns the log in case it can be of futher use
async fn log_message_history<'a, T: Write>(
    data: &'a BotData, 
    guild_id: GuildId, 
    channel_id: ChannelId, 
    message_id: MessageId, 
    mut w: T,
) -> anyhow::Result<Vec<MessageLog>> {
    let log = data.get_message_log(guild_id, channel_id, message_id).await?;
    let log_len = log.len();
    if log_len > 0 {
        write!(w, "*Bot recorded message history:*\n")?;     
    }

    for entry in log.iter() {        
        let prefix = match entry.type_ {
            LogType::Create => "Create",
            LogType::Edit => "Edit",
            LogType::Delete => "Delete",
        };

        let timestamp = entry.timestamp.timestamp();
        let content = entry
            .message
            .as_ref()
            .map(|v| message_to_string(v))
            .transpose()?
            .flatten()
            .map(|v| v.replace("\n", "\n> "));

        let user_id = entry.user_id
            .map(|v| v.0);

        write!(w, "<t:{timestamp}>:  **{prefix}**")?;
        if let Some(user_id) = user_id {
            write!(w, " (<@{user_id}>)")?;
        }
        if let Some(content) = content {
            write!(w, ":\n> {content}\n")?;
        } else {
            write!(w, "\n")?;
        }
    }

    Ok(log)
}

async fn handle_message_component_interaction<'a>(ctx: &serenity::Context, data: &'a BotData, interaction: &'a Interaction) -> anyhow::Result<()> {
    let message_component = if let Interaction::MessageComponent(mc) = interaction {
        mc
    } else {
        return Ok(());
    };

    if message_component.guild_id.is_none() ||
       message_component.data.component_type != ComponentType::Button 
    {        
        return Ok(());
    }

    fn split_custom_id(custom_id: Option<&str>) -> anyhow::Result<(String, RoleId)> {
        let custom_id = custom_id.ok_or(anyhow::anyhow!("Interaction custom_id missing"))?;
        let parts = custom_id
            .split(INTERACTION_BUTTON_CUSTOM_ID_DELIMITER)
            .collect::<Vec<_>>();

        if parts.len() == 3 && parts[0] == INTERACTION_BUTTON_CUSTOM_ID_PREFIX {
            Ok((parts[1].to_string(), RoleId::from(parts[2].parse::<u64>()?)))
        } else {
            Err(anyhow::anyhow!("Interaction custom_id didn't match the expected format"))
        }
    }

    let (name, _) = split_custom_id(Some(message_component.data.custom_id.as_str()))?;

    let guild_id = message_component.guild_id.unwrap();
    
    let message = &message_component.message;
    let embed = if message.embeds.len() == 1 {
        &message.embeds[0]
    } else {
        Err(anyhow::anyhow!("Button interaction with more than one embed. Don't know how to parse that"))?
    };
    let timestamp = interaction.id().created_at();

    let mut choices = Vec::new();
    for button in message.components
        .iter()
        .map(|row| row
            .components
            .iter())
        .flatten()
        .filter_map(|component| if let &ActionRowComponent::Button(ref b) = component {
            Some(b)
        } else {
            None
        })
    {
        let choice = button.label.as_ref().ok_or(anyhow::anyhow!("Interaction button with no label not supported"))?.clone();
        let (_, role_id) = split_custom_id(button.custom_id.as_ref().map(|v| v.as_str()))?;
        let emoji = button.emoji.as_ref().map(|v| format!("{}", v));

        choices.push((choice, role_id, emoji));
    }

    let mut ir = None;
    for i in 0..3 {
        ir = data.get_interaction_role(guild_id.into(), name.clone()).await?;
        
        match (i, ir.is_some()) {
            (0, false) => {
                data.update_interaction_role(
                    guild_id.into(), 
                    name.clone(), 
                    embed.description.clone(), 
                    message.channel_id.into(), 
                    Some(message.id.into()),
                    false, 
                    timestamp,
                ).await?;
            },
            (_, true) => {
                let ir = ir.as_ref().unwrap();
                choices.retain(|(choice, _, _)| !ir.choices.iter().any(|v| &v.choice == choice));

                if choices.len() > 0 {
                    // Create the choices
                    for (choice, role_id, emoji) in choices.iter() {                    
                        data.update_interaction_choice(
                            guild_id.into(), 
                            name.clone(), 
                            choice.clone(), 
                            emoji.clone(), 
                            *role_id, 
                            timestamp,
                        ).await?;                 
                    }   
                } else {
                    // No need to fetch again
                    break;
                }                
            },
            (_, false) => Err(anyhow::anyhow!("Failed to create interaction role from button interaction"))?,
        }
    }
    
    message_component.create_interaction_response(&ctx.http, |b| b
        .interaction_response_data(|b| b
            .ephemeral(true)
            .embed(|b|                 
                Embed::default()
                    .description(format!("{:#?}", ir))
                    .create_embed(b)
    )))
    .await?;

    Ok(())
}