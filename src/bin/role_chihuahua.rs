use std::{num::ParseIntError, time::Duration};

use chrono::Utc;
use clap::Parser;
use futures::future::join;
use gagbot_rs::{
    commands::greet::{run_greet, GreetBehaviour},
    db::{
        queries::{
            config::{self}, self,
        },
        DbCommand,
    },
    *,
};
use poise::{
    self,
    serenity_prelude::{
        self as serenity,  Context, GatewayIntents,
        Guild,
    },
    FrameworkContext, FrameworkError,
};
use tracing::*;

fn frequency_seconds_valid_range(s: &str) -> Result<u64, String> {
    let v = s.parse().map_err(|e: ParseIntError| e.to_string())?;

    if v < 60 {
        Err(format!(
            "Running more often than once per minute ({} seconds) is not recommended",
            v
        ))?;
    }
    Ok(v)
}

#[derive(Debug, Parser)]
#[clap(name = "gagbot.rs")]
struct Cli {
    #[clap(long, env)]
    chihuahua_discord_token: String,
    #[clap(long, env, default_value = "gagbot.sqlite")]
    sqlite_connection_string: String,
    #[clap(long, env, default_value = "64")]
    database_command_channel_bound: usize,
    #[clap(long, env, default_value = "3600", value_parser = frequency_seconds_valid_range)]
    background_task_frequency_seconds: u64,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    load_dotenv()?;
    configure_tracing();

    let args = Cli::parse();
    let background_task_frequency = Duration::from_secs(args.background_task_frequency_seconds);
    let discord_token = &args.chihuahua_discord_token;
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
                            // TODO: Most of these should probably only be implemented on the main bot and/or shared
                            DbCommand::Optimize { respond_to } => {
                                respond_to.send(optimize_database(&sqlite_con))
                                    .map_err(|_| anyhow::anyhow!("Optimize respond_to oneshot closed"))?;
                            },
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
                            DbCommand::GetTableBytesAndCount { respond_to } => {
                                respond_to.send(queries::get_table_size_in_bytes(&sqlite_con))
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
        debug!("DB TASK: exiting");

        close_database(sqlite_con)?;

        Ok::<_, anyhow::Error>(())
    });

    let options = poise::FrameworkOptions {
        commands: discord_commands::chihuahua_commands(),
        on_error: |err| Box::pin(on_error(err)),
        event_handler: |ctx, event, framework, data| {
            Box::pin(event_handler(ctx, event, framework, data))
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .client_settings(|b| b.cache_settings(|s| s.max_messages(CACHE_MAX_MESSAGES)))
        .options(options)
        .token(discord_token)
        .intents(GatewayIntents::GUILDS | GatewayIntents::GUILD_MEMBERS)
        .setup(move |_ctx, _ready, _framework| {
            debug!("Discord connected");
            Box::pin(async move {
                Ok(BotData::new(
                    sender,
                    db_file_path,
                    background_task_frequency,
                ))
            })
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

async fn on_error(error: FrameworkError<'_, BotData, PoiseError>) {
    let msg = match &error {
        FrameworkError::ArgumentParse {
            error, ..
        } => Some(error.to_string()),
        FrameworkError::Command {
            error, ..
        } => {
            // TODO: Probably do something more fancy with user facing errors and the like?
            Some(error.to_string())
        }

        e => {
            warn!("UNHANDLED error from poise: {:?}", e);
            None
        }
    };

    if let (Some(msg), Some(ctx)) = (msg, error.ctx().as_ref()) {
        if let Err(e) = Embed::error().description(msg).send(ctx).await {
            error!("Error from ctx.send in error handler: {:?}", e);
        }
    }
}

async fn event_handler<'a>(
    ctx: &serenity::Context,
    event: &'a poise::Event<'a>,
    framework: FrameworkContext<'a, BotData, PoiseError>,
    data: &'a BotData,
) -> Result<(), PoiseError> {
    use poise::Event::*;

    debug!("got event: {}", event.name());
    trace!("EVENT VALUE: {:#?}", event);
    match event {
        GuildCreate {
            guild, ..
        } => handle_guild_create(ctx, data, framework, guild).await?,
        GuildMemberAddition {
            new_member,
        } => handle_guild_member_add(data, ctx, new_member).await?,
        _ => {}
    }

    Ok(())
}

async fn handle_guild_create<'a>(
    ctx: &Context,
    data: &BotData,
    framework: FrameworkContext<'a, BotData, PoiseError>,
    guild: &Guild,
) -> Result<(), Error> {
    poise::builtins::register_in_guild(ctx, &framework.options().commands, guild.id)        
        .await
        .context("register_in_guild")?;

    let channel_id = data
        .general_log_channel(guild.id.into())
        .await?
        .or(guild.system_channel_id.map(|v| v.into()))
        .or(guild
            .default_channel_guaranteed()
            .map(|c| c.id)
            .map(|v| v.into()));
    if let Some(chan) = channel_id {
        let now = Utc::now().timestamp();
        Embed::default()
            .random_color()
            .title("I'm here")
            .description(format!("<t:{}>\nVersion: {}", now, env!("CARGO_PKG_VERSION")))
            .send_in_channel(chan, &ctx.http)
            .await
            .context("send_in_channel (I'm here)")?;
    } else {
        warn!("Failed to get log, system or default channels to log to");
    }
    Ok(())
}

async fn handle_guild_member_add(
    data: &BotData,
    ctx: &Context,
    new_member: &serenity::Member,
) -> Result<(), Error> {
    let guild_id = new_member.guild_id;    
    run_greet(&data, &ctx, guild_id.into(), new_member.clone(), GreetBehaviour::ApplyRole).await?;
    Ok(())
}