use std::{fmt::Debug, os::unix::fs::MetadataExt, path::PathBuf, time::Duration};

use db::{
    queries::{
        config::{ConfigKey, LogChannel},
        interaction_roles::InteractionRole,
        message_log::{LogType, MessageLog},
        permissions::{EffectivePermission, Permission},
    }, CompressionState, DbCommand
};
use lazy_regex::{regex, Captures};
use poise::serenity_prelude::{Guild, Member, Message, Timestamp, User};
use tokio::sync::oneshot;

mod ids;
pub use ids::*;

pub mod db;

mod error;
pub use error::*;

mod embed;
pub use embed::*;
use tracing_subscriber::fmt::format::FmtSpan;

pub mod commands;
pub mod discord_commands;

pub const GAGBOT_ICON: &str = "https://cdn.discordapp.com/emojis/708352151558029322.png";
pub const GAGBOT_ICON_ERROR: &str = "https://cdn.discordapp.com/emojis/708352247804854285.png";

pub const GAGBOT_COLOR_NORMAL: i32 = 0xEBC634;
pub const GAGBOT_COLOR_ERROR: i32 = 0xFF0000;
pub const GAGBOT_COLOR_SUCCESS: i32 = 0x00FF00;
pub const GAGBOT_COLOR_LOG_EDIT: i32 = 0x30649c;
pub const GAGBOT_COLOR_LOG_DELETE: i32 = 0x9c3730;
pub const GAGBOT_COLOR_GREET: i32 = 0x65e7b7;
pub const GAGBOT_COLOR_WELCOME: i32 = GAGBOT_COLOR_GREET;
pub const GAGBOT_COLOR_LOG_JOIN: i32 = 0x009900;
pub const GAGBOT_COLOR_LOG_LEAVE: i32 = 0x990044;

pub const INTERACTION_BUTTON_CUSTOM_ID_MAX_LEN: usize = 100;
pub const INTERACTION_BUTTON_CUSTOM_ID_ROLE_ID_MAX_LEN: usize = 21;
pub const INTERACTION_BUTTON_CUSTOM_ID_DELIMITER: char = '¬';
pub const INTERACTION_BUTTON_CUSTOM_ID_PREFIX: &str = "rr";
pub const INTERACTION_BUTTON_CUSTOM_ID_NAME_MAX_LEN: usize = INTERACTION_BUTTON_CUSTOM_ID_MAX_LEN
    - INTERACTION_BUTTON_CUSTOM_ID_ROLE_ID_MAX_LEN
    - 1 //delimiter
    - INTERACTION_BUTTON_CUSTOM_ID_PREFIX.len();

/// The edit tracking functionality won't work without some cached messages
/// 200 is the default from discord.js <https://github.com/discordjs/discord.js/blob/86e5f5a119c6d2588b988a33236d358ded357847/packages/discord.js/src/util/Options.js#L175>
pub const CACHE_MAX_MESSAGES: usize = 200;

pub const DISK_SPACE_WARNING_LEVEL: u64 = 5 * 1024 * 1024 * 1024;

pub fn configure_tracing() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_line_number(true)
            .with_file(true)
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .finish(),
    )
    .expect("Failed to set default tracing subscriber");
}

#[derive(Debug, Clone)]
pub struct BotData {
    pub db_command_sender: flume::Sender<DbCommand>,
    pub db_file_path: Option<PathBuf>,
    pub background_task_frequency: Duration,
}

impl BotData {
    pub fn new(
        db_command_sender: flume::Sender<DbCommand>,
        db_file_path: Option<PathBuf>,
        background_task_frequency: Duration,
    ) -> Self {
        Self {
            db_command_sender,
            db_file_path,
            background_task_frequency,
        }
    }

    pub fn db_available_space(&self) -> Result<u64, Error> {
        if self.db_file_path.is_none() {
            Err(anyhow::anyhow!("DB appears to not be disk backed? Can't check the available space"))?;
        }

        Ok(fs2::available_space(self.db_file_path.as_ref().unwrap())?)
    }

    pub fn db_file_size(&self) -> Result<u64, Error> {
        if self.db_file_path.is_none() {
            Err(anyhow::anyhow!("DB appears to not be disk backed? Can't check the available space"))?;
        }

        Ok(self.db_file_path.as_ref().unwrap().metadata()?.size())
    }

    pub async fn general_log_channel(&self, guild_id: GuildId) -> Result<Option<ChannelId>, Error> {
        self.log_channel(guild_id, vec![LogChannel::General]).await
    }

    pub async fn general_log_channel_or_default(&self, guild: &Guild) -> Result<Option<ChannelId>, Error> {
        Ok(self.general_log_channel(guild.id.into())
            .await?
            .or(guild.system_channel_id.map(|v| v.into()))
            .or(guild
                .default_channel_guaranteed()
                .map(|c| c.id)
                .map(|v| v.into())))
    }

    pub async fn error_log_channel(&self, guild_id: GuildId) -> Result<Option<ChannelId>, Error> {
        self.log_channel(guild_id, vec![LogChannel::Error, LogChannel::General])
            .await
    }

    pub async fn log_channel(
        &self,
        guild_id: GuildId,
        purpose: Vec<LogChannel>,
    ) -> Result<Option<ChannelId>, Error> {
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
    ) -> Result<usize, Error> {
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
    ) -> Result<Option<(ChannelId, Embed)>, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetGreet {
                guild_id,
                respond_to: s,
            })
            .await?;

        if let Some((channel_id, mut message)) = r.await?? {
            expand_greeting_template(user, &mut message);

            let mut embed = Embed::default()
                .content(format!("{user}"))
                .description(message)
                .random_color();
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
    ) -> Result<(), Error> {
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

    pub async fn set_config(
        &self,
        guild_id: GuildId,
        key: ConfigKey,
        timestamp: Timestamp,
        value: String,
    ) -> Result<(), Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::SetConfigString {
                guild_id,
                key,
                value,
                timestamp,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn get_member_permissions(
        &self,
        guild: &Guild,
        member: &Member,
    ) -> Result<Vec<EffectivePermission>, Error> {
        let sorted_roles = {
            let mut roles = guild
                .roles
                .values()
                // Filter the roles down to only the ones the member has
                .filter_map(|v| {
                    if member.roles.contains(&v.id) {
                        Some((v.position, v.id))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            // Sort by position so higher roles are on the top
            roles.sort_by(|(a, _), (b, _)| b.cmp(a));

            roles.into_iter().map(|(_, b)| b.into()).collect::<Vec<_>>()
        };

        let guild_id = member.guild_id.into();

        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetMemberPermissions {
                guild_id,
                sorted_roles,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn require_permission(
        &self,
        guild: &Guild,
        member: &Member,
        permission: Permission,
    ) -> Result<EffectivePermission, Error> {
        let effective_permission = self
            .get_member_permissions(guild, member)
            .await?
            .into_iter()
            // TODO: do we need anything more sophisticated like a tree of permissions?
            .find(|x| x.permission == permission || x.permission == Permission::All);

        effective_permission.ok_or(Error::PermissionDenied(permission).into())
    }

    pub async fn grant_permission(
        &self,
        guild_id: GuildId,
        role_id: RoleId,
        permission: Permission,
        timestamp: Timestamp,
    ) -> Result<bool, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GrantPermission {
                guild_id,
                role_id,
                permission,
                timestamp,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn revoke_permission(
        &self,
        guild_id: GuildId,
        role_id: RoleId,
        permission: Permission,
        timestamp: Timestamp,
    ) -> Result<bool, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::RevokePermission {
                guild_id,
                role_id,
                permission,
                timestamp,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn purge_permission(&self, guild_id: GuildId, timestamp: Timestamp) -> Result<bool, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::PurgePermissions {
                guild_id,
                timestamp,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn get_interaction_role(
        &self,
        guild_id: GuildId,
        name: String,
    ) -> Result<Option<InteractionRole>, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetInteractionRole {
                guild_id,
                name,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn update_interaction_role(
        &self,
        guild_id: GuildId,
        name: String,
        description: Option<String>,
        channel_id: ChannelId,
        message_id: Option<MessageId>,
        exclusive: bool,
        timestamp: Timestamp,
    ) -> Result<bool, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::UpdateInteractionRoleSet {
                guild_id,
                name,
                description,
                channel_id,
                message_id,
                exclusive,
                timestamp,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn update_interaction_choice(
        &self,
        guild_id: GuildId,
        set_name: String,
        choice: String,
        emoji: Option<String>,
        role_id: RoleId,
        timestamp: Timestamp,
    ) -> Result<bool, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::UpdateInteractionRoleChoice {
                guild_id,
                set_name,
                choice,
                emoji,
                role_id,
                timestamp,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn log_message(
        &self,
        message_id: MessageId,
        timestamp: Timestamp,
        type_: LogType,
        message: Option<Message>,
    ) -> Result<(), Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::LogMessage {
                message_id,
                timestamp,
                type_,
                message,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn get_message_log(
        &self,
        message_id: MessageId,
    ) -> Result<Vec<MessageLog>, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetLogMessages {
                message_id,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    // pub async fn lookup_user_from_message(
    //     &self,
    //     guild_id: GuildId,
    //     channel_id: ChannelId,
    //     message_id: MessageId,
    // ) -> Result<Option<UserId>, Error> {
    //     let (s, r) = oneshot::channel();
    //     self.db_command_sender
    //         .send_async(DbCommand::GetUserFromLogMessages {
    //             guild_id,
    //             channel_id,
    //             message_id,
    //             respond_to: s,
    //         })
    //         .await?;
    //     Ok(r.await??)
    // }

    pub async fn db_table_sizes(&self) -> Result<Vec<(String, u64, u64)>, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetTableBytesAndCount {
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn db_compression_state(&self) -> Result<CompressionState, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetCompressionState {
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn db_optimize(&self) -> Result<Duration, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::Optimize { 
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn get_config_u64(&self, guild_id: GuildId, key: ConfigKey) -> Result<Option<u64>, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetConfigU64 {
                guild_id,
                key,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn get_config_string(
        &self,
        guild_id: GuildId,
        key: ConfigKey,
    ) -> Result<Option<String>, Error> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetConfigString {
                guild_id,
                key,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }
}

pub fn expand_greeting_template(user: &User, message: &mut String) {
    let replace_regex = regex!(r"\{\{([^{}]+)}}");
    *message = replace_regex.replace_all(&message, |caps: &Captures| match &caps[0] {
        "{{tag}}" => user.to_string(),
        "{{name}}" => user.name.clone(),
        "{{discriminator}}" => user.discriminator.to_string(),
        _ => format!("{{{{ unknown replacement \"{}\" }}", &caps[0]),
    })
    .replace("\\n", "\n").to_string();
}

pub type PoiseError = Error;//Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, BotData, Error>;



pub fn load_dotenv() -> Result<Option<PathBuf>, dotenv::Error> {
    match dotenv::dotenv() {
        // Swallow NotFound error since the .env is optional
        Err(dotenv::Error::Io(ref e)) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        r => r.map(|p| Some(p)),
    }
}