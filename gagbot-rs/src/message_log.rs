use poise::serenity_prelude::{Timestamp};
use rusqlite::{params, Connection, ToSql, types::{ToSqlOutput, FromSql, FromSqlError, ValueRef, FromSqlResult}, OptionalExtension};
use tracing::error;
use std::str;
use crate::{ChannelId, GuildId, UserId, MessageId};


#[derive(Debug)]
pub enum LogType {
    Create,
    Edit,
    Delete,
}

#[derive(Debug)]
pub struct MessageLog {
    pub guild_id: GuildId,
    pub user_id: Option<UserId>,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub timestamp: Timestamp,
    pub type_: LogType,
    pub content: Option<String>,
}

impl ToSql for LogType {
    fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
        Ok(match self {
            LogType::Create => ToSqlOutput::Borrowed("CREATE".into()),
            LogType::Edit => ToSqlOutput::Borrowed("EDIT".into()),
            LogType::Delete => ToSqlOutput::Borrowed("DELETE".into()),
        })
    }
}

impl FromSql for LogType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        if let ValueRef::Text(v) = value {
            match str::from_utf8(v).map_err(|e| FromSqlError::Other(Box::new(e)))? {
                "CREATE" => Ok(LogType::Create),
                "EDIT" => Ok(LogType::Edit),
                "DELETE" => Ok(LogType::Delete),
                e => {
                    error!("Unexpected enum variant {} for LogType", e);
                    Err(FromSqlError::InvalidType)
                },
            }
        } else {
            Err(FromSqlError::InvalidType)
        }
    }
}


pub fn log(
    db: &Connection,
    guild_id: GuildId,
    user_id: Option<UserId>,
    channel_id: ChannelId,
    message_id: MessageId,
    timestamp: Timestamp,
    type_: LogType,
    content: Option<String>,
) -> anyhow::Result<()> {
    let mut stmt = db.prepare_cached(
        "INSERT INTO message_log (guild_id, user_id, channel_id, message_id, timestamp, type, content)
        VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;

    stmt.execute(params![guild_id, user_id, channel_id, message_id, &timestamp.to_rfc3339(), type_, content])?;

    Ok(())
}

pub fn get(
    db: &Connection,
    guild_id: GuildId,
    channel_id: ChannelId,
    message_id: MessageId,
) -> anyhow::Result<Vec<MessageLog>> {
    let mut stmt = db.prepare_cached(
        "SELECT guild_id, user_id, channel_id, message_id, timestamp, type, content FROM message_log
        WHERE guild_id = ?1 AND channel_id = ?2 AND message_id = ?3
        ORDER BY timestamp DESC",
    )?;

    let r = stmt.query_map(
        params![guild_id, channel_id, message_id],
        |r| Ok(MessageLog {
            guild_id: GuildId::from(r.get::<_, u64>(0)?),
            user_id: r.get::<_, Option<u64>>(1)?.map(|v| UserId::from(v)),
            channel_id: r.get(2)?,
            message_id: MessageId::from(r.get::<_, u64>(3)?),
            timestamp: Timestamp::from(r.get::<_, String>(4)?),
            type_: r.get(5)?,
            content: r.get(6)?,
        }))?.collect::<Result<_, _>>()?;

    Ok(r)
}

pub fn get_user(
    db: &Connection,
    guild_id: GuildId,
    channel_id: ChannelId,
    message_id: MessageId,
) -> anyhow::Result<Option<UserId>> {
    let mut stmt = db.prepare_cached(
        "SELECT user_id FROM message_log
        WHERE guild_id = ?1 AND channel_id = ?2 AND message_id = ?3 AND user_id IS NOT NULL
        LIMIT 1
        ORDER BY timestamp DESC",
    )?;

    let r = stmt
        .query_row(params![guild_id, channel_id, message_id], |r| r.get::<_, u64>(0))
        .optional()?
        .map(|id| UserId::from(id));

    Ok(r)
}
