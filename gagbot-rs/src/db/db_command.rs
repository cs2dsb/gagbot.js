use anyhow::Result;
use poise::serenity_prelude::{Message, Timestamp};
use tokio::sync::oneshot::Sender;

use crate::{
    db::queries::config::{ConfigKey, LogChannel},
    interaction_roles::InteractionRole,
    message_log::{LogType, MessageLog},
    permissions::{EffectivePermission, Permission},
    ChannelId, GuildId, MessageId, RoleId, UserId,
};

#[derive(Debug)]
pub enum DbCommand {
    GetGreet {
        guild_id: GuildId,
        respond_to: Sender<Result<Option<(ChannelId, String)>>>,
    },
    GetConfigString {
        guild_id: GuildId,
        key: ConfigKey,
        respond_to: Sender<Result<Option<String>>>,
    },
    GetConfigI64 {
        guild_id: GuildId,
        key: ConfigKey,
        respond_to: Sender<Result<Option<i64>>>,
    },
    GetConfigU64 {
        guild_id: GuildId,
        key: ConfigKey,
        respond_to: Sender<Result<Option<u64>>>,
    },
    SetConfigString {
        guild_id: GuildId,
        key: ConfigKey,
        value: String,
        timestamp: Timestamp,
        respond_to: Sender<Result<()>>,
    },
    DeleteConfig {
        guild_id: GuildId,
        key: ConfigKey,
        timestamp: Timestamp,
        respond_to: Sender<Result<()>>,
    },
    GetLogChannel {
        guild_id: GuildId,
        purpose: Vec<LogChannel>,
        respond_to: Sender<Result<Option<ChannelId>>>,
    },
    GetMessageCount {
        guild_id: GuildId,
        user_id: UserId,
        channel_id: Option<ChannelId>,
        respond_to: Sender<Result<usize>>,
    },
    IncrementMessageCount {
        guild_id: GuildId,
        user_id: UserId,
        channel_id: ChannelId,
        respond_to: Sender<Result<()>>,
    },
    GetMemberPermissions {
        guild_id: GuildId,
        sorted_roles: Vec<RoleId>,
        respond_to: Sender<Result<Vec<EffectivePermission>>>,
    },
    GrantPermission {
        guild_id: GuildId,
        role_id: RoleId,
        permission: Permission,
        timestamp: Timestamp,
        respond_to: Sender<Result<bool>>,
    },
    RevokePermission {
        guild_id: GuildId,
        role_id: RoleId,
        permission: Permission,
        timestamp: Timestamp,
        respond_to: Sender<Result<bool>>,
    },
    PurgePermissions {
        guild_id: GuildId,
        timestamp: Timestamp,
        respond_to: Sender<Result<bool>>,
    },
    UpdateInteractionRoleSet {
        guild_id: GuildId,
        name: String,
        description: Option<String>,
        channel_id: ChannelId,
        message_id: Option<MessageId>,
        exclusive: bool,
        timestamp: Timestamp,
        respond_to: Sender<Result<bool>>,
    },
    UpdateInteractionRoleChoice {
        guild_id: GuildId,
        set_name: String,
        choice: String,
        emoji: Option<String>,
        role_id: RoleId,
        timestamp: Timestamp,
        respond_to: Sender<Result<bool>>,
    },
    GetInteractionRole {
        guild_id: GuildId,
        name: String,
        respond_to: Sender<Result<Option<InteractionRole>>>,
    },
    LogMessage {
        guild_id: GuildId,
        user_id: Option<UserId>,
        channel_id: ChannelId,
        message_id: MessageId,
        timestamp: Timestamp,
        type_: LogType,
        message: Option<Message>,
        respond_to: Sender<Result<()>>,
    },
    GetLogMessages {
        guild_id: GuildId,
        channel_id: ChannelId,
        message_id: MessageId,
        respond_to: Sender<Result<Vec<MessageLog>>>,
    },
    GetUserFromLogMessages {
        guild_id: GuildId,
        channel_id: ChannelId,
        message_id: MessageId,
        respond_to: Sender<Result<Option<UserId>>>,
    },
    GetTableBytesAndCount {
        respond_to: Sender<Result<Vec<(String, u64, u64)>>>,
    },
}
