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
use gagbot_rs::{config::LogChannel, *, message_log::LogType};
use poise::{self, FrameworkContext, FrameworkError, serenity_prelude::{self as serenity, ActionRowComponent, CacheHttp, ComponentType, GatewayIntents, Interaction, Timestamp, Message}};
use tracing::*;
use std::fmt::Write;

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
                            DbCommand::LogMessage { guild_id, user_id, channel_id, message_id, timestamp, type_, content, respond_to } => {
                                respond_to.send(message_log::log(&sqlite_con, guild_id, user_id, channel_id, message_id, timestamp, type_, content))
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
    let bot_id = framework.bot_id;

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
                
                if user_id != bot_id {
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
                        message_to_string(new_message)?,
                    ).await?;
                }
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

                data.log_message(
                    guild_id.into(), 
                    Some(new.author.id.into()), 
                    new.channel_id.into(),
                    new.id.into(),
                    event.edited_timestamp.unwrap_or(Timestamp::now()),
                    LogType::Edit,
                    message_to_string(new)?,
                ).await?;
            }
        }
        MessageDelete {
            channel_id,
            deleted_message_id,
            guild_id,
        } => {
            if let Some(guild_id) = guild_id {
                // Log to the log channel only if it is configured
                if let Some(log_channel_id) = data
                    .log_channel((*guild_id).into(), vec![LogChannel::EditsAndDeletes])
                    .await?
                {
                    let mut is_bot = false;
                    let mut user = None;
                    let mut msg = String::new();

                    // This attempts to get it from the cache
                    if let Some(cache) = ctx.cache() {
                        if let Some(message) = cache.message(channel_id, deleted_message_id) {
                            let content = message_to_string(&message)?.unwrap_or(" ".to_string());
                            is_bot = message.author.bot;
                            user = Some(message.author);
                            if !is_bot {
                                write!(&mut msg, "```\n{}\n```", content)?;
                            }
                        } else {
                            warn!(
                                "Failed to look up deleted_message_id ({}/{}) from cache for guild {}",
                                channel_id, deleted_message_id, guild_id
                            );
                        }
                    }

                    if user.is_none() {
                        match data.lookup_user_from_message(
                            guild_id.into(), 
                            channel_id.into(), 
                            deleted_message_id.into(),
                        ).await {
                            Ok(Some(user_id)) => match ctx.http.get_user(user_id.into()).await {
                                Ok(user_) => user = Some(user_),
                                Err(e) => error!("Error getting user from user_id {:?} from discord: {:?}", user_id, e),
                            },
                            Ok(None) => warn!("Failed to find a user_id for message {} from the message log", deleted_message_id),
                            Err(e) => error!("Error looking up user_id from message log: {:?}", e),
                        }
                    }

                    // Log to the DB, we always do this regardless of config
                    data.log_message::<String>(
                        guild_id.into(), 
                        user.as_ref().map(|u| u.id.into()), 
                        channel_id.into(),
                        deleted_message_id.into(),
                        // Seems like serenity should provide the timestamp of the event from discord but it doesn't seem to
                        Timestamp::now(),
                        LogType::Delete,
                        None,
                    ).await?;

                    if !is_bot {
                        log_message_history(data, guild_id.into(), channel_id.into(), deleted_message_id.into(), &mut msg).await?;

                        Embed::default()
                            .flavour(EmbedFlavour::LogDelete)
                            .description(msg)
                            .title(if let Some(user) = user {
                                format!("Message from {} was deleted", user.tag())
                            } else {
                                "Message was deleted (Original user data is unknown)".to_string()
                            })
                            .send_in_channel(log_channel_id, &ctx.http)
                            .await?;
                    }
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
        InteractionCreate { interaction } => {
            handle_message_component_interaction(ctx, data, interaction).await?;            
        }
        _ => {}
    }

    Ok(())
}

fn message_to_string(message: &Message) -> anyhow::Result<Option<String>> {
    let mut content = message.content.clone();

    for e in message.embeds.iter() {
        content.push_str("Embed {\n");
        if let Some(v) = e.title.as_ref() {
            write!(&mut content, "\ttitle: \"{v}\",\n")?;
        }
        if let Some(v) = e.description.as_ref() {
            write!(&mut content, "\tdescription: \"{v}\",\n")?;
        }
        content.push_str("}\n");
    }

    for a in message.attachments.iter() {
        content.push_str("Attachment {\n");
        write!(&mut content, "\tfilename: \"{}\",\n", a.filename)?;        
        if let Some(v) = a.content_type.as_ref() {
            write!(&mut content, "\tcontent_type: \"{v}\",\n")?;
        }        
        write!(&mut content, "\turl: \"{}\",\n", a.url)?;        
        content.push_str("}\n");
    }    

    let content = if content.len() > 0 {
        Some(content)
    } else {
        debug!("message_to_string empty message: {:#?}", message);
        None
    };

    Ok(content)
}

async fn log_message_history<'a, T: Write>(data: &'a BotData, guild_id: GuildId, channel_id: ChannelId, message_id: MessageId, mut w: T) -> anyhow::Result<()> {
    let log = data.get_message_log(guild_id, channel_id, message_id).await?;
    let log_len = log.len();
    if log_len > 0 {
        write!(w, "\nBot recorded message history:\n```")?;     
    }

    for entry in log {        
        let prefix = match entry.type_ {
            LogType::Create => "Create",
            LogType::Edit => "Edit",
            LogType::Delete => "Delete",
        };

        let timestamp = entry.timestamp;
        let content = entry
            .content
            .unwrap_or_default()
            .replace("\n", "\n\t");
        let user_id = entry.user_id.map_or("<Unknown>".to_string(), |v| v.to_string());

        write!(w, "{prefix} {{\n\ttimestamp: {timestamp}\n\tuser_id: <@{user_id}>\n\tcontent: {content}\n}}\n")?;
    }

    if log_len > 0 {
        write!(w, "```")?;
    }

    Ok(())
}

async fn handle_message_component_interaction<'a>(ctx: &serenity::Context, data: &'a BotData, interaction: &'a Interaction) -> anyhow::Result<Option<String>> {
    let message_component = if let Interaction::MessageComponent(mc) = interaction {
        mc
    } else {
        return Ok(None);
    };

    if message_component.guild_id.is_none() ||
       message_component.data.component_type != ComponentType::Button 
    {        
        return Ok(None);
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

    Ok(None)
}