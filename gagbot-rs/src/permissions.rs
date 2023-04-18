// use poise::serenity_prelude::Timestamp;
use rusqlite::{types::ToSqlOutput, /* Connection, */ ToSql};
// use tracing::debug;

// pub const CONFIG_KEY_GREET_MESSAGE: &str = "greet.message";

#[derive(Debug)]
pub enum PermissionType {
    Role,
    User,
}

impl ToSql for PermissionType {
    fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
        Ok(match self {
            PermissionType::Role => ToSqlOutput::Borrowed("ROLE".into()),
            PermissionType::User => ToSqlOutput::Borrowed("USER".into()),
        })
    }
}

// TODO: there's some nuance around this to work out what to do when removing a
// permission that was granted by a wildcard. TBD pub fn update_permission(db:
// &Connection, guild_id: &str, role_or_user_id: &str, type_: PermissionType,
// value: &str, timestamp: &Timestamp) -> anyhow::Result<()> {     let mut stmt
// = db.prepare_cached(         "INSERT INTO config (guild_id, discord_id, type,
// value, last_updated)                 VALUES (?1, ?2, ?3, ?4, ?5)
//              ON CONFLICT(guild_id, discord_id, type, value) DO UPDATE SET
//                 value = excluded.value,
//                 last_updated = excluded.last_updated
//             WHERE excluded.last_updated > last_updated")?;

//     match stmt.execute(&[guild_id, role_or_user_id, type_, value,
// &timestamp.to_rfc3339()]) {         Ok(1) => {
//             debug!("Permission value for {} updated successfully", key);
//             Ok(())
//         },
//         Ok(_) => {
//             debug!("Config value for {} not updated because database version
// is newer", key);             Err(anyhow::anyhow!("Config value for {} not
// updated because database version is newer", key))         },
//         Err(e) => {
//             debug!("Error updating config value for {}: {:}", key, e);
//             Err(e)?
//         },
//     }
//     unimplemented!()
// }
