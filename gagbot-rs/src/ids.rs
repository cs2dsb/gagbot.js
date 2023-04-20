use std::{ops::Deref, str::FromStr, str};
use rusqlite::{types::{ToSqlOutput, FromSql, ValueRef, FromSqlResult, FromSqlError}, ToSql};

macro_rules! wrap_id {
    ($wrapper:ident) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $wrapper(poise::serenity_prelude::model::id::$wrapper);
        impl ToSql for $wrapper {
            fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
                self.0 .0.to_sql()
            }
        }
        impl From<poise::serenity_prelude::model::id::$wrapper> for $wrapper {
            fn from(value: poise::serenity_prelude::model::id::$wrapper) -> Self {
                Self(value)
            }
        }
        impl From<u64> for $wrapper {
            fn from(value: u64) -> Self {
                Self(poise::serenity_prelude::model::id::$wrapper(value))
            }
        }
        impl Deref for $wrapper {
            type Target = poise::serenity_prelude::model::id::$wrapper;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

    };
}

macro_rules! id_from_str {
    ($wrapper:ident) => {
        impl FromStr for $wrapper {
            type Err = <poise::serenity_prelude::model::id::$wrapper as FromStr>::Err;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                poise::serenity_prelude::model::id::$wrapper::from_str(s).map(|v| Self(v))
            }
        }

        impl FromSql for $wrapper {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                match value {
                    ValueRef::Integer(v) => Ok(poise::serenity_prelude::model::id::$wrapper::from(v as u64).into()),
                    ValueRef::Real(v) => Ok(poise::serenity_prelude::model::id::$wrapper::from(v as u64).into()),
                    ValueRef::Text(v) => poise::serenity_prelude::model::id::$wrapper::from_str(
                        str::from_utf8(v)
                            .map_err(|e| FromSqlError::Other(Box::new(e)))?
                    )
                    .map(|v| Self(v))
                    .map_err(|e| FromSqlError::Other(Box::new(e))),
                    _ => Err(FromSqlError::InvalidType),
                }
            }
        }
    };
}

wrap_id!(RoleId);
id_from_str!(RoleId);

wrap_id!(GuildId);

wrap_id!(ChannelId);
id_from_str!(ChannelId);

wrap_id!(UserId);