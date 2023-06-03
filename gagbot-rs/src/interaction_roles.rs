use poise::serenity_prelude::Timestamp;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{ChannelId, GuildId, MessageId, RoleId};

#[derive(Debug, Clone)]
pub struct InteractionRole {
    pub guild_id: GuildId,
    pub name: String,
    pub description: Option<String>,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub exclusive: bool,
    pub choices: Vec<InteractionChoice>,
}

#[derive(Debug, Clone)]
pub struct InteractionChoice {
    pub guild_id: GuildId,
    pub choice: String,
    pub emoji: Option<String>,
    pub role_id: RoleId,
}

pub fn get(
    db: &Connection,
    guild_id: GuildId,
    name: String,
) -> anyhow::Result<Option<InteractionRole>> {
    let mut stmt = db.prepare_cached(
        "SELECT name, description, channel_id, message_id, exclusive FROM interaction_role 
            WHERE guild_id = ?1 AND name = ?2",
    )?;

    if let Some(mut ir) = stmt
        .query_row(params![guild_id, &name], |r| {
            Ok(InteractionRole {
                guild_id,
                name: r.get(0)?,
                description: r.get(1)?,
                channel_id: r.get(2)?,
                message_id: MessageId::from(r.get::<_, u64>(3)?),
                exclusive: r.get(4)?,
                choices: Vec::new(),
            })
        })
        .optional()?
    {
        let mut stmt = db.prepare_cached(
            "SELECT choice, emoji, role_id FROM interaction_role_choice 
                WHERE guild_id = ?1 AND set_name = ?2",
        )?;

        for choice in stmt.query_map(params![guild_id, &name], |r| {
            Ok(InteractionChoice {
                guild_id,
                choice: r.get(0)?,
                emoji: r.get(1)?,
                role_id: RoleId::from(r.get::<_, u64>(2)?),
            })
        })? {
            ir.choices.push(choice?);
        }

        Ok(Some(ir))
    } else {
        Ok(None)
    }
}

pub fn update(
    db: &Connection,
    guild_id: GuildId,
    name: String,
    description: Option<String>,
    channel: ChannelId,
    message_id: Option<MessageId>,
    exclusive: bool,
    timestamp: Timestamp,
) -> anyhow::Result<bool> {
    let mut stmt = db.prepare_cached(
        "INSERT INTO interaction_role (guild_id, name, description, channel_id, message_id, exclusive, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(guild_id, name) DO UPDATE SET
                description = excluded.description, 
                channel_id = excluded.channel_id, 
                message_id = excluded.message_id, 
                exclusive = excluded.exclusive,
                last_updated = excluded.last_updated
            WHERE excluded.last_updated > last_updated",
    )?;

    stmt.execute(params![
        guild_id,
        name,
        description,
        channel,
        message_id,
        exclusive,
        &timestamp.to_rfc3339()
    ])?;

    Ok(true)
}

pub fn update_choice(
    db: &Connection,
    guild_id: GuildId,
    set_name: String,
    choice: String,
    emoji: Option<String>,
    role_id: RoleId,
    timestamp: Timestamp,
) -> anyhow::Result<bool> {
    let mut stmt = db.prepare_cached(
        "INSERT INTO interaction_role_choice (guild_id, set_name, choice, emoji, role_id, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(guild_id, set_name, choice) DO UPDATE SET
                choice = excluded.choice,
                emoji = excluded.emoji,
                last_updated = excluded.last_updated,
                role_id = excluded.role_id
            WHERE excluded.last_updated > last_updated",
    )?;

    stmt.execute(params![
        guild_id,
        set_name,
        choice,
        emoji,
        role_id,
        &timestamp.to_rfc3339()
    ])?;

    Ok(true)
}
