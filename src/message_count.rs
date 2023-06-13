use rusqlite::{params, Connection};

use crate::{ChannelId, GuildId, UserId, Error};

pub fn increment(
    db: &Connection,
    guild_id: GuildId,
    user_id: UserId,
    channel_id: ChannelId,
) -> Result<(), Error> {
    let mut stmt = db.prepare_cached(
        "INSERT INTO message_count (guild_id, user_id, channel_id, message_count) 
        VALUES(?1, ?2, ?3, 1)
        ON CONFLICT(guild_id, user_id, channel_id) DO UPDATE 
        SET message_count = message_count + 1",
    )?;

    stmt.execute(params![guild_id, user_id, channel_id])?;

    Ok(())
}

pub fn get(
    db: &Connection,
    guild_id: GuildId,
    user_id: UserId,
    channel_id: Option<ChannelId>,
) -> Result<usize, Error> {
    let mut stmt = db.prepare_cached(if channel_id.is_some() {
        "SELECT COALESCE(SUM(message_count), 0) FROM message_count 
        WHERE guild_id=?1 AND user_id=?2 and channel_id=?3"
    } else {
        "SELECT COALESCE(SUM(message_count), 0) FROM message_count 
        WHERE guild_id=?1 AND user_id=?2"
    })?;

    let count = if channel_id.is_some() {
        stmt.query_row(params![guild_id, user_id, channel_id], |r| {
            r.get::<_, usize>(0)
        })?
    } else {
        stmt.query_row(params![guild_id, user_id], |r| r.get::<_, usize>(0))?
    };

    Ok(count)
}
