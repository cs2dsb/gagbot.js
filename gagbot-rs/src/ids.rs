use std::{ops::Deref, str::FromStr};

use poise::serenity_prelude::model::id::{
    ChannelId as SChannelId, GuildId as SGuildId, UserId as SUserId,
};
use rusqlite::{types::ToSqlOutput, ToSql};

#[derive(Debug, Clone, Copy)]
pub struct GuildId(SGuildId);
impl ToSql for GuildId {
    fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
        self.0 .0.to_sql()
    }
}
impl From<SGuildId> for GuildId {
    fn from(value: SGuildId) -> Self {
        Self(value)
    }
}
impl From<u64> for GuildId {
    fn from(value: u64) -> Self {
        Self(SGuildId(value))
    }
}
impl Deref for GuildId {
    type Target = SGuildId;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChannelId(SChannelId);
impl ToSql for ChannelId {
    fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
        self.0 .0.to_sql()
    }
}
impl FromStr for ChannelId {
    type Err = <SChannelId as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SChannelId::from_str(s).map(|v| Self(v))
    }
}
impl From<SChannelId> for ChannelId {
    fn from(value: SChannelId) -> Self {
        Self(value)
    }
}
impl From<u64> for ChannelId {
    fn from(value: u64) -> Self {
        Self(SChannelId(value))
    }
}
impl Deref for ChannelId {
    type Target = SChannelId;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UserId(SUserId);
impl ToSql for UserId {
    fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
        self.0 .0.to_sql()
    }
}
impl From<SUserId> for UserId {
    fn from(value: SUserId) -> Self {
        Self(value)
    }
}
impl From<u64> for UserId {
    fn from(value: u64) -> Self {
        Self(SUserId(value))
    }
}
impl Deref for UserId {
    type Target = SUserId;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
