use std::{
    fmt::{Display, Write},
    str::{self, FromStr},
};

use anyhow::Context as _;

use async_trait::async_trait;
use poise::{serenity_prelude::Timestamp, ChoiceParameter};
use rusqlite::{
    params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef},
    Connection, ToSql,
};
use tracing::debug;

use crate::{Context, GuildId, RoleId};

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

#[async_trait]
pub trait PermissionCheck {
    async fn require_permission(self, permission: Permission) -> anyhow::Result<()>;
}

#[async_trait]
impl<'a> PermissionCheck for &'a Context<'a> {
    async fn require_permission(self, permission: Permission) -> anyhow::Result<()> {
        let guild = self
            .guild()
            .ok_or(anyhow::anyhow!("missing guild in require_permission"))?;
        let caller = self.author_member().await.ok_or(anyhow::anyhow!(
            "missing author_member in require_permission"
        ))?;

        if guild.owner_id != caller.user.id {
            self.data()
                .require_permission(&guild, &caller, permission)
                .await
                // TODO: better logging around this - errors here just come out as a message "require_permissions"
                //       nothing in the log, nothing else in the message... not good
                .context("require_permission")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, ChoiceParameter, PartialEq)]
pub enum Permission {
    #[name = "! All"]
    All,

    #[name = "permissions.manage"]
    PermissionManage,

    #[name = "config.manage"]
    ConfigManage,

    #[name = "messages.purge"]
    MessagePurge,

    #[name = "member.add"]
    MemberAdd,

    #[name = "member.promote"]
    MemberPromote,
}

impl ToSql for Permission {
    fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
        self.name().to_sql()
    }
}

impl FromSql for Permission {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        if let ValueRef::Text(v) = value {
            Ok(
                Self::from_str(str::from_utf8(v).map_err(|e| FromSqlError::Other(Box::new(e)))?)
                    .map_err(|e| FromSqlError::Other(Box::new(e)))?,
            )
        } else {
            Err(FromSqlError::InvalidType)
        }
    }
}

#[derive(Debug, Clone)]
pub struct EffectivePermission {
    pub role: RoleId,
    pub permission: Permission,
}

impl Display for EffectivePermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<@&{}>: {}", self.role.0, self.permission)
    }
}

pub fn get(
    db: &Connection,
    guild_id: GuildId,
    sorted_roles: Vec<RoleId>,
) -> anyhow::Result<Vec<EffectivePermission>> {
    if sorted_roles.len() == 0 {
        return Ok(vec![]);
    }

    let mut sql = "SELECT discord_id, value FROM permission WHERE guild_id=?1 AND type=?2 AND discord_id IN (".to_string();

    let p_i = 3;
    for i in 0..sorted_roles.len() {
        if i > 0 {
            sql.push_str(", ");
        }
        write!(&mut sql, "?{}", i + p_i)?;
    }

    sql.push_str(") ORDER BY CASE ");

    for i in 0..sorted_roles.len() {
        write!(&mut sql, "WHEN discord_id=?{} THEN {} ", i + p_i, i + 1)?;
    }
    write!(&mut sql, "ELSE {} END", sorted_roles.len() + 1)?;

    // I think it's worth caching this even though it is highly dynamic because
    // some of the tasks mods/admins do with the bot will require running several
    // commands back-to-back and the permission check+role count will be the same
    let mut stmt = db.prepare_cached(&sql)?;

    let mut params: Vec<&dyn ToSql> = vec![&guild_id, &PermissionType::Role];
    for r in sorted_roles.iter() {
        params.push(r);
    }

    let permissions = stmt
        .query_map(params.as_slice(), |row| {
            Ok(EffectivePermission {
                role: row.get(0)?,
                permission: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(permissions)
}

pub fn grant(
    db: &Connection,
    guild_id: GuildId,
    role_id: RoleId,
    permission: Permission,
    timestamp: Timestamp,
) -> anyhow::Result<bool> {
    let mut stmt = db.prepare_cached(
        "INSERT INTO permission (guild_id, discord_id, type, value, last_updated)
                             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(guild_id, discord_id, type, value) DO NOTHING",
    )?;

    match stmt.execute(params![
        guild_id,
        role_id,
        PermissionType::Role,
        permission,
        &timestamp.to_rfc3339()
    ]) {
        Ok(1) => {
            debug!("Permission {} granted", permission);
            Ok(true)
        }
        Ok(_) => {
            debug!("Permission {} already granted", permission);
            Ok(false)
        }
        Err(e) => {
            debug!("Error granting permission {}: {:}", permission, e);
            Err(e)?
        }
    }
}

pub fn revoke(
    db: &Connection,
    guild_id: GuildId,
    role_id: RoleId,
    permission: Permission,
    timestamp: Timestamp,
) -> anyhow::Result<bool> {
    let mut stmt = db.prepare_cached(
        "DELETE FROM permission 
             WHERE guild_id=?1 AND discord_id=?2 AND type=?3 AND value=?4
               AND last_update < ?5",
    )?;

    match stmt.execute(params![
        guild_id,
        role_id,
        PermissionType::Role,
        permission,
        &timestamp.to_rfc3339()
    ]) {
        Ok(1) => {
            debug!("Permission {} revoked", permission);
            Ok(true)
        }
        Ok(_) => {
            debug!("Permission {} was already revoked", permission);
            Ok(false)
        }
        Err(e) => {
            debug!("Error revoking permission {}: {:}", permission, e);
            Err(e)?
        }
    }
}

pub fn purge(db: &mut Connection, guild_id: GuildId, timestamp: Timestamp) -> anyhow::Result<bool> {
    let tx = db.transaction()?;

    {
        let mut stmt = tx.prepare(
            "SELECT COUNT(CASE WHEN last_updated >= ?2 THEN 1 END) AS NewRows,
            COUNT(CASE WHEN last_updated < ?2 THEN 1 END) AS OldRows
            FROM permission
            WHERE guild_id = ?1",
        )?;

        match stmt.query_row(params![guild_id, &timestamp.to_rfc3339()], |row| {
            Ok((row.get::<_, usize>(0)?, row.get::<_, usize>(1)?))
        }) {
            Ok((0, 0)) => return Ok(false),
            Ok((0, _)) => {}
            Ok((_, _)) => Err(anyhow::anyhow!(
                "Purge cancelled as data has been updated since the purge command was issued"
            ))?,
            Err(e) => Err(e)?,
        };
    }

    {
        let mut stmt = tx.prepare(
            "DELETE FROM permission 
            WHERE guild_id=?1",
        )?;
        if let Err(e) = stmt.execute(params![guild_id]) {
            Err(e)?;
        }
    }
    tx.commit()?;
    Ok(true)
}
