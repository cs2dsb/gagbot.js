use poise::serenity_prelude::Timestamp;
use tokio::sync::oneshot;

use crate::{
    config::{ConfigKey, LogChannel},
    ChannelId, GuildId, UserId,
};

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
}
