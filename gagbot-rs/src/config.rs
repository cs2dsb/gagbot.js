use std::{any::type_name, str::FromStr};

use anyhow::Context;
use poise::{serenity_prelude::Timestamp, ChoiceParameter};
use rusqlite::{params, types::ToSqlOutput, Connection, OptionalExtension, ToSql};
use tracing::debug;

use crate::{ChannelId, GuildId};

#[derive(Debug, Clone, Copy, ChoiceParameter)]
pub enum ConfigKey {
    #[name = "greet.message"]
    GreetMessage,
    #[name = "greet.channel"]
    GreetChannel,
    #[name = "greet.role"]
    GreetRole,
    #[name = "greet.welcomemessage"]
    GreetWelcomeMessage,
    #[name = "greet.welcomechannel"]
    GreetWelcomeChannel,
    #[name = "promote.new_chat_channel"]
    PromoteNewChatChannel,
    #[name = "promote.junior_chat_channel"]
    PromoteJuniorChatChannel,
    #[name = "promote.junior_role"]
    PromoteJuniorRole,
    #[name = "promote.full_role"]
    PromoteFullRole,
    #[name = "promote.new_chat_min_messages"]
    PromoteNewChatMinMessages,
    #[name = "promote.junior_chat_min_messages"]
    PromoteJuniorChatMinMessages,
    #[name = "promote.junior_min_age"]
    PromoteJuniorMinAge,
    #[name = "promote.new_message_max_age"]
    PromoteNewMessageMaxAge,
    #[name = "logging.general"]
    LoggingGeneral,
    #[name = "logging.edits_and_deletes"]
    LoggingEditsAndDeletes,
    #[name = "logging.joining_and_leaving"]
    LoggingJoiningAndLeaving,
    #[name = "logging.errors"]
    LoggingErrors,
    #[name = "logging.voice_activity"]
    LoggingVoiceActivity,
}

impl ConfigKey {
    pub fn logging_keys() -> &'static [Self] {
        &[
            ConfigKey::LoggingGeneral,
            ConfigKey::LoggingEditsAndDeletes,
            ConfigKey::LoggingJoiningAndLeaving,
            ConfigKey::LoggingErrors,
            ConfigKey::LoggingVoiceActivity,
        ]
    }
}

impl ToSql for ConfigKey {
    fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
        self.name().to_sql()
    }
}

#[derive(Debug, PartialEq, Eq, ChoiceParameter)]
pub enum LogChannel {
    General,
    Error,
    EditsAndDeletes,
    JoiningAndLeaving,
    VoiceActivity,
}

impl ToSql for LogChannel {
    fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
        Ok(match self {
            LogChannel::General => ToSqlOutput::Borrowed(ConfigKey::LoggingGeneral.name().into()),
            LogChannel::Error => ToSqlOutput::Borrowed(ConfigKey::LoggingErrors.name().into()),
            LogChannel::EditsAndDeletes => {
                ToSqlOutput::Borrowed(ConfigKey::LoggingEditsAndDeletes.name().into())
            }
            LogChannel::JoiningAndLeaving => {
                ToSqlOutput::Borrowed(ConfigKey::LoggingJoiningAndLeaving.name().into())
            }
            LogChannel::VoiceActivity => {
                ToSqlOutput::Borrowed(ConfigKey::LoggingVoiceActivity.name().into())
            }
        })
    }
}

pub fn update(
    db: &Connection,
    guild_id: GuildId,
    key: ConfigKey,
    value: &str,
    timestamp: Timestamp,
) -> anyhow::Result<()> {
    let mut stmt = db.prepare_cached(
        "INSERT INTO config (guild_id, key, value, last_updated)
                         VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(guild_id, key) DO UPDATE SET
                value = excluded.value,
                last_updated = excluded.last_updated
            WHERE excluded.last_updated > last_updated",
    )?;

    match stmt.execute(params![guild_id, key, value, &timestamp.to_rfc3339()]) {
        Ok(1) => {
            debug!("Config value for {} updated successfully", key);
            Ok(())
        }
        Ok(_) => {
            debug!(
                "Config value for {} not updated because database version is newer",
                key
            );
            Err(anyhow::anyhow!(
                "Config value for {} not updated because database version is newer",
                key
            ))
        }
        Err(e) => {
            debug!("Error updating config value for {}: {:}", key, e);
            Err(e)?
        }
    }
}

pub fn delete(
    db: &Connection,
    guild_id: GuildId,
    key: ConfigKey,
    timestamp: Timestamp,
) -> anyhow::Result<()> {
    let mut stmt = db.prepare_cached(
        "DELETE FROM config 
             WHERE guild_id=?1 AND key=?2 AND last_updated<?3",
    )?;

    match stmt.execute(params![guild_id, key, &timestamp.to_rfc3339()]) {
        Ok(1) => {
            debug!("Config value for {} deleted successfully", key);
            Ok(())
        }
        Ok(_) => {
            let mut stmt =
                db.prepare("SELECT last_updated FROM config WHERE guild_id=?1 AND key=?2")?;
            if let Ok(last_updated) =
                stmt.query_row(params![guild_id, key], |r| r.get::<_, String>(0))
            {
                let err = format!(
                    "Config value for {} not deleted because it was updated after delete command was issued ({})",                    
                    key,
                    last_updated,
                );
                debug!("{}", err);
                Err(anyhow::anyhow!("{}", err))
            } else {
                // There was nothing to delete
                Ok(())
            }
        }
        Err(e) => {
            debug!("Error deleting config value for {}: {}", key, e);
            Err(e)?
        }
    }
}

pub fn get<T>(db: &Connection, guild_id: GuildId, key: ConfigKey) -> anyhow::Result<Option<T>>
where
    T: FromStr,
    <T as FromStr>::Err: 'static + Send + Sync + std::error::Error,
{
    // TODO: currently internal db errors are munged together with missing data
    // errors       should probably have some Internal vs User visible error
    // split
    let mut stmt = db.prepare_cached(
        "SELECT value FROM config WHERE 
            guild_id = ?1 AND
            key = ?2",
    )?;

    let value = stmt
        .query_row(params![guild_id, key], |r| r.get::<_, String>(0))
        .optional()
        .with_context(|| format!("Failed to get config value for {}", key))?
        .map(|v| {
            v.parse().with_context(|| {
                format!(
                    "Failed to parse config value for {} as {}",
                    key,
                    type_name::<T>()
                )
            })
        })
        .transpose()?;

    Ok(value)
}

/// Attempts to find the best log channel by looking for each `purposes` value
/// in sequence Returns None if none of the are configured
pub fn get_log_channel(
    db: &Connection,
    guild_id: GuildId,
    purposes: &[LogChannel],
) -> anyhow::Result<Option<ChannelId>> {
    let mut stmt = db.prepare_cached(
        "SELECT value 
        FROM config 
        WHERE guild_id=?1 
        AND key=?2",
    )?;

    for p in purposes.iter() {
        if let Some(value) = stmt
            .query_row(params![guild_id, p], |r| r.get::<_, String>(0))
            .optional()?
        {
            return Ok(Some(value.parse().with_context(|| {
                format!("Failed to parse ChannelId from \"{}\"", value)
            })?));
        }
    }

    Ok(None)
}

pub fn get_greet(
    db: &Connection,
    guild_id: GuildId,
) -> anyhow::Result<Option<(ChannelId, String)>> {
    let channel: Option<ChannelId> = get(db, guild_id, ConfigKey::GreetChannel)?;
    if channel.is_none() {
        return Ok(None);
    }

    let message: Option<String> = get(db, guild_id, ConfigKey::GreetMessage)?;
    if message.is_none() {
        return Ok(None);
    }

    Ok(Some((channel.unwrap(), message.unwrap())))
}
