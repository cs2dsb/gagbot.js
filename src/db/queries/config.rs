use std::{any::type_name, str::FromStr};

use poise::{serenity_prelude::Timestamp, ChoiceParameter};
use rusqlite::{params, types::ToSqlOutput, Connection, OptionalExtension, ToSql};
use tracing::debug;

use crate::{ChannelId, GuildId, ErrorContext, Error};

#[derive(Debug, Clone, Copy, ChoiceParameter)]
pub enum ConfigKey {
    #[name = "greet.message"]
    GreetMessage,
    #[name = "greet.channel"]
    GreetChannel,
    #[name = "greet.role"]
    GreetRole,
    #[name = "greet.default_role"]
    GreetDefaultRole,
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

    pub fn description(&self) -> &'static str {
        match self {
            ConfigKey::GreetMessage => "Message template bot posts to new members. Use {tag}, {name} and {discriminator} to refer to the new member",
            ConfigKey::GreetChannel => "Channel to post the greeting in",
            ConfigKey::GreetWelcomeMessage => "Message to post in welcome channel after new member has been approved by a mod",
            ConfigKey::GreetWelcomeChannel => "Channel to post welcome in",
            ConfigKey::LoggingGeneral => "Channel to log general bot messages in",
            ConfigKey::LoggingEditsAndDeletes => "Channel to log message edits and deletes in",
            ConfigKey::LoggingJoiningAndLeaving => "Channel to log join and leave events in",
            ConfigKey::LoggingErrors => "Channel to log bot errors in",
            ConfigKey::LoggingVoiceActivity => "Channel to log member voice activity in",
            ConfigKey::GreetRole => "Role given by a mod as part of the add member process",
            ConfigKey::GreetDefaultRole => "Role given automatically when a member joins the server",
            ConfigKey::PromoteJuniorRole => "Role given once an introduction has been done",
            ConfigKey::PromoteFullRole => "Role given after a certain time has passed and number of messages are posted",
            ConfigKey::PromoteNewChatChannel => "Channel new members must post their introduction in",
            ConfigKey::PromoteJuniorChatChannel => "Channel juniors have to be active in",
            ConfigKey::PromoteNewChatMinMessages => "How many messages new members have to post in into channel",
            ConfigKey::PromoteJuniorChatMinMessages => "How many messages juniors have to post to show they are active",
            ConfigKey::PromoteJuniorMinAge => "How long (in days) juniors have to stick around to be promoted",
        }
    }
}

impl ToSql for ConfigKey {
    fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
        self.name().to_sql()
    }
}

#[derive(Debug, PartialEq, Eq, ChoiceParameter, Clone, Copy)]
pub enum LogChannel {
    General,
    Error,
    EditsAndDeletes,
    JoiningAndLeaving,
    VoiceActivity,
}

impl Into<ConfigKey> for LogChannel {
    fn into(self) -> ConfigKey {
        match self {
            Self::General => ConfigKey::LoggingGeneral,
            Self::EditsAndDeletes => ConfigKey::LoggingEditsAndDeletes,
            Self::JoiningAndLeaving => ConfigKey::LoggingJoiningAndLeaving,
            Self::Error => ConfigKey::LoggingErrors,
            Self::VoiceActivity => ConfigKey::LoggingVoiceActivity,
        }
    }
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
) -> Result<(), Error> {
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
            let msg = format!("Config value for {} not updated because database version is newer", key);
            debug!(msg);
            Err(anyhow::anyhow!(msg))?
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
) -> Result<(), Error> {
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
                Err(anyhow::anyhow!("{}", err))?
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

pub fn get<T>(db: &Connection, guild_id: GuildId, key: ConfigKey) -> Result<Option<T>, Error>
where
    T: FromStr,
    <T as FromStr>::Err: Into<Error>,
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
) -> Result<Option<ChannelId>, Error> {
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
) -> Result<Option<(ChannelId, String)>, Error> {
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
