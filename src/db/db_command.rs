use std::time::Duration;

use poise::serenity_prelude::{Message, Timestamp};
use tokio::sync::oneshot::Sender;

use crate::{
    db::queries::{
        config::{ConfigKey, LogChannel},
        interaction_roles::InteractionRole,
        message_log::{LogType, MessageLog},
        permissions::{EffectivePermission, Permission},
    },
    ChannelId, GuildId, MessageId, RoleId, UserId, Error
};


pub type CommandSender = flume::Sender<DbCommand>;
pub type CommandReceiver = flume::Receiver<DbCommand>;

#[derive(Debug, Clone)]
pub struct CompressionState {
    pub uncompressed_messages: u64,
    pub uncompressed_bytes: u64,
    pub compressed_messages: u64,
    pub compressed_bytes: u64,
    pub chunks: u64,
}

#[derive(Debug, strum::Display)]
pub enum DbCommand {
    GetCompressionState {
        respond_to: Sender<Result<CompressionState, Error>>,
    },
    Optimize {
        respond_to: Sender<Result<Duration, Error>>,
    },
    Vacuum {
        respond_to: Sender<Result<Duration, Error>>,
    },
    Compress {
        respond_to: Sender<Result<(Duration, bool), Error>>,
    },
    GetGreet {
        guild_id: GuildId,
        respond_to: Sender<Result<Option<(ChannelId, String)>, Error>>,
    },
    GetConfigString {
        guild_id: GuildId,
        key: ConfigKey,
        respond_to: Sender<Result<Option<String>, Error>>,
    },
    GetConfigI64 {
        guild_id: GuildId,
        key: ConfigKey,
        respond_to: Sender<Result<Option<i64>, Error>>,
    },
    GetConfigU64 {
        guild_id: GuildId,
        key: ConfigKey,
        respond_to: Sender<Result<Option<u64>, Error>>,
    },
    SetConfigString {
        guild_id: GuildId,
        key: ConfigKey,
        value: String,
        timestamp: Timestamp,
        respond_to: Sender<Result<(), Error>>,
    },
    DeleteConfig {
        guild_id: GuildId,
        key: ConfigKey,
        timestamp: Timestamp,
        respond_to: Sender<Result<(), Error>>,
    },
    GetLogChannel {
        guild_id: GuildId,
        purpose: Vec<LogChannel>,
        respond_to: Sender<Result<Option<ChannelId>, Error>>,
    },
    GetMessageCount {
        guild_id: GuildId,
        user_id: UserId,
        channel_id: Option<ChannelId>,
        respond_to: Sender<Result<usize, Error>>,
    },
    IncrementMessageCount {
        guild_id: GuildId,
        user_id: UserId,
        channel_id: ChannelId,
        respond_to: Sender<Result<(), Error>>,
    },
    GetMemberPermissions {
        guild_id: GuildId,
        sorted_roles: Vec<RoleId>,
        respond_to: Sender<Result<Vec<EffectivePermission>, Error>>,
    },
    GrantPermission {
        guild_id: GuildId,
        role_id: RoleId,
        permission: Permission,
        timestamp: Timestamp,
        respond_to: Sender<Result<bool, Error>>,
    },
    RevokePermission {
        guild_id: GuildId,
        role_id: RoleId,
        permission: Permission,
        timestamp: Timestamp,
        respond_to: Sender<Result<bool, Error>>,
    },
    PurgePermissions {
        guild_id: GuildId,
        timestamp: Timestamp,
        respond_to: Sender<Result<bool, Error>>,
    },
    UpdateInteractionRoleSet {
        guild_id: GuildId,
        name: String,
        description: Option<String>,
        channel_id: ChannelId,
        message_id: Option<MessageId>,
        exclusive: bool,
        timestamp: Timestamp,
        respond_to: Sender<Result<bool, Error>>,
    },
    UpdateInteractionRoleChoice {
        guild_id: GuildId,
        set_name: String,
        choice: String,
        emoji: Option<String>,
        role_id: RoleId,
        timestamp: Timestamp,
        respond_to: Sender<Result<bool, Error>>,
    },
    GetInteractionRole {
        guild_id: GuildId,
        name: String,
        respond_to: Sender<Result<Option<InteractionRole>, Error>>,
    },
    LogMessage {
        message_id: MessageId,
        timestamp: Timestamp,
        type_: LogType,
        message: Option<Message>,
        respond_to: Sender<Result<(), Error>>,
    },
    GetLogMessages {
        message_id: MessageId,
        respond_to: Sender<Result<Vec<MessageLog>, Error>>,
    },
    // GetUserFromLogMessages {
    //     guild_id: GuildId,
    //     channel_id: ChannelId,
    //     message_id: MessageId,
    //     respond_to: Sender<Result<Option<UserId>, Error>>,
    // },
    GetTableBytesAndCount {
        respond_to: Sender<Result<Vec<(String, u64, u64)>, Error>>,
    },
}
