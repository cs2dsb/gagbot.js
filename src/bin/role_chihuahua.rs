use std::{num::ParseIntError, time::Duration};

use chrono::Utc;
use clap::Parser;
use futures::future::join;
use gagbot_rs::{
    commands::greet::{run_greet, GreetBehaviour},
    db::{
        open_database, spawn_db_task, DbCommand
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
    let sqlite_con = open_database(&args.sqlite_connection_string, true, false)?;
    let db_file_path = sqlite_con.path().map(|p| p.to_owned());
    let (sender, receiver) = flume::bounded::<DbCommand>(args.database_command_channel_bound);

    
    let db_task_handle = spawn_db_task(sqlite_con, receiver);

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