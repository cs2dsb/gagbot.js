use anyhow::Result;
use config::LogChannel;
use include_dir::{include_dir, Dir};
use lazy_regex::{regex, Captures};
use poise::serenity_prelude::User;
use rusqlite::{Connection, OpenFlags};
use rusqlite_migration::Migrations;
use tokio::sync::oneshot;

pub mod config;
pub mod message_count;

mod ids;
pub use ids::*;

mod db;
pub use db::*;

mod permissions;
pub use permissions::*;

mod embed;
pub use embed::*;

pub mod commands;

pub const GAGBOT_ICON: &str = "https://cdn.discordapp.com/emojis/708352151558029322.png";
pub const GAGBOT_ICON_ERROR: &str = "https://cdn.discordapp.com/emojis/708352247804854285.png";

pub const GAGBOT_COLOR_NORMAL: i32 = 0xEBC634;
pub const GAGBOT_COLOR_ERROR: i32 = 0xFF0000;
pub const GAGBOT_COLOR_SUCCESS: i32 = 0x00FF00;
pub const GAGBOT_COLOR_LOG_EDIT: i32 = 0x30649c;
pub const GAGBOT_COLOR_LOG_DELETE: i32 = 0x9c3730;
pub const GAGBOT_COLOR_GREET: i32 = 0x65e7b7;

/// The edit tracking functionality won't work without some cached messages
/// 200 is the default from discord.js <https://github.com/discordjs/discord.js/blob/86e5f5a119c6d2588b988a33236d358ded357847/packages/discord.js/src/util/Options.js#L175>
pub const CACHE_MAX_MESSAGES: usize = 200;

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

#[derive(Debug)]
pub struct BotData {
    pub db_command_sender: flume::Sender<DbCommand>,
}

impl BotData {
    pub fn new(db_command_sender: flume::Sender<DbCommand>) -> Self {
        Self {
            db_command_sender,
        }
    }

    pub async fn general_log_channel(&self, guild_id: GuildId) -> Result<Option<ChannelId>> {
        self.log_channel(guild_id, vec![config::LogChannel::General])
            .await
    }

    pub async fn error_log_channel(&self, guild_id: GuildId) -> Result<Option<ChannelId>> {
        self.log_channel(
            guild_id,
            vec![config::LogChannel::Error, config::LogChannel::General],
        )
        .await
    }

    pub async fn log_channel(
        &self,
        guild_id: GuildId,
        purpose: Vec<LogChannel>,
    ) -> Result<Option<ChannelId>> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetLogChannel {
                guild_id,
                purpose,
                respond_to: s,
            })
            .await?;
        Ok(r.await??.map(|v| v.into()))
    }

    pub async fn message_count(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        channel_id: Option<ChannelId>,
    ) -> Result<usize> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetMessageCount {
                guild_id,
                user_id,
                channel_id: channel_id,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn get_greet(
        &self,
        guild_id: GuildId,
        user: &User,
    ) -> Result<Option<(ChannelId, Embed)>> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetGreet {
                guild_id,
                respond_to: s,
            })
            .await?;

        if let Some((channel_id, message)) = r.await?? {
            let replace_regex = regex!(r"\{\{([^{}]+)}}");
            let user = std::sync::Arc::new(user);
            let message = replace_regex.replace_all(&message, |caps: &Captures| match &caps[0] {
                "{{tag}}" => user.to_string(),
                "{{name}}" => user.name.clone(),
                "{{discriminator}}" => user.discriminator.to_string(),
                _ => format!("{{{{ unknown replacement \"{}\" }}", &caps[0]),
            });
            let message = message.replace("\\n", "\n").to_string();

            let mut embed = Embed::default()
                .description(message)
                .color(GAGBOT_COLOR_GREET);
            embed.thumbnail_url = user.avatar_url();

            Ok(Some((channel_id, embed)))
        } else {
            Ok(None)
        }
    }

    pub async fn increment_message_count(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<()> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::IncrementMessageCount {
                guild_id,
                user_id,
                channel_id,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, BotData, Error>;

pub fn configure_tracing() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_line_number(true)
            .with_file(true)
            .finish(),
    )
    .expect("Failed to set default tracing subscriber");
}

pub fn open_database(connection_string: &str, create: bool) -> Result<Connection> {
    let mut open_flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;

    if create {
        open_flags |= OpenFlags::SQLITE_OPEN_CREATE;
    }

    let mut con = Connection::open_with_flags(connection_string, open_flags)?;

    let migrations = Migrations::from_directory(&MIGRATIONS_DIR)?;
    migrations.to_latest(&mut con)?;

    con.pragma_update(None, "journal_mode", "WAL")?;
    con.pragma_update(None, "synchronous", "NORMAL")?;
    con.pragma_update(None, "foreign_keys", "ON")?;

    Ok(con)
}

pub fn close_database(con: Connection) -> Result<()> {
    con.pragma_update(None, "analysis_limit", "400")?;
    con.pragma_update(None, "optimize", "")?;
    con.pragma_update(None, "foreign_keys", "ON")?;

    if let Err((_con, e)) = con.close() {
        Err(e)?;
    }

    Ok(())
}
