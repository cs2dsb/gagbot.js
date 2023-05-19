use poise::serenity_prelude::{ Timestamp };
use rusqlite::Connection;
use tokio::sync::oneshot;

use crate::{MessageId, ChannelId, GuildId, RoleId, UserId, config::{ConfigKey, LogChannel}, interaction_roles::InteractionRole, permissions::{EffectivePermission, Permission}, message_log::{LogType, MessageLog}};

#[derive(Debug)]
pub enum DbCommand {
    GetGreet {
        guild_id: GuildId,
        respond_to: oneshot::Sender<anyhow::Result<Option<(ChannelId, String)>>>,
    },
    GetConfigString {
        guild_id: GuildId,
        key: ConfigKey,
        respond_to: oneshot::Sender<anyhow::Result<Option<String>>>,
    },
    GetConfigI64 {
        guild_id: GuildId,
        key: ConfigKey,
        respond_to: oneshot::Sender<anyhow::Result<Option<i64>>>,
    },
    SetConfigString {
        guild_id: GuildId,
        key: ConfigKey,
        value: String,
        timestamp: Timestamp,
        respond_to: oneshot::Sender<anyhow::Result<()>>,
    },
    DeleteConfig {
        guild_id: GuildId,
        key: ConfigKey,
        timestamp: Timestamp,
        respond_to: oneshot::Sender<anyhow::Result<()>>,
    },
    GetLogChannel {
        guild_id: GuildId,
        purpose: Vec<LogChannel>,
        respond_to: oneshot::Sender<anyhow::Result<Option<ChannelId>>>,
    },
    GetMessageCount {
        guild_id: GuildId,
        user_id: UserId,
        channel_id: Option<ChannelId>,
        respond_to: oneshot::Sender<anyhow::Result<usize>>,
    },
    IncrementMessageCount {
        guild_id: GuildId,
        user_id: UserId,
        channel_id: ChannelId,
        respond_to: oneshot::Sender<anyhow::Result<()>>,
    },
    GetMemberPermissions {
        guild_id: GuildId,
        sorted_roles: Vec<RoleId>,
        respond_to: oneshot::Sender<anyhow::Result<Vec<EffectivePermission>>>,
    },
    GrantPermission {
        guild_id: GuildId,
        role_id: RoleId,
        permission: Permission,
        timestamp: Timestamp,
        respond_to: oneshot::Sender<anyhow::Result<bool>>,
    },
    RevokePermission {
        guild_id: GuildId,
        role_id: RoleId,
        permission: Permission,
        timestamp: Timestamp,
        respond_to: oneshot::Sender<anyhow::Result<bool>>,
    },
    PurgePermissions {
        guild_id: GuildId,
        timestamp: Timestamp,
        respond_to: oneshot::Sender<anyhow::Result<bool>>,
    },
    UpdateInteractionRoleSet {
        guild_id: GuildId,
        name: String,
        description: Option<String>,
        channel_id: ChannelId,
        message_id: Option<MessageId>,
        exclusive: bool,
        timestamp: Timestamp,
        respond_to: oneshot::Sender<anyhow::Result<bool>>,
    },
    UpdateInteractionRoleChoice {
        guild_id: GuildId,
        set_name: String,
        choice: String,
        emoji: Option<String>,
        role_id: RoleId,
        timestamp: Timestamp,
        respond_to: oneshot::Sender<anyhow::Result<bool>>,
    },
    GetInteractionRole {
        guild_id: GuildId,
        name: String,
        respond_to: oneshot::Sender<anyhow::Result<Option<InteractionRole>>>,
    },
    LogMessage {
        guild_id: GuildId,
        user_id: Option<UserId>,
        channel_id: ChannelId,
        message_id: MessageId,
        timestamp: Timestamp,
        type_: LogType,
        content: Option<String>,
        respond_to: oneshot::Sender<anyhow::Result<()>>,
    },
    GetLogMessages {
        guild_id: GuildId,
        channel_id: ChannelId,
        message_id: MessageId,
        respond_to: oneshot::Sender<anyhow::Result<Vec<MessageLog>>>,
    },
    GetUserFromLogMessages {
        guild_id: GuildId,
        channel_id: ChannelId,
        message_id: MessageId,
        respond_to: oneshot::Sender<anyhow::Result<Option<UserId>>>,
    },
    GetTableBytes {
        respond_to: oneshot::Sender<anyhow::Result<Vec<(String, u64)>>>,
    }
}

pub fn get_table_bytes(db: &Connection) -> anyhow::Result<Vec<(String, u64)>> {
    let mut stmt = db.prepare("SELECT name, SUM(pgsize) FROM dbstat GROUP BY name")?;

    let r = stmt.query_map([],
        |r| Ok((r.get(0)?, r.get(1)?))
    )?.collect::<Result<_, _>>()?;

    Ok(r)
}