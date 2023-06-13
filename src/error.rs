use std::fmt;
use crate::{permissions::Permission, db::DbCommand};

pub const OBFUSCATED_ERROR_MSG: &str = "An error occured";

pub trait ErrorContext<T, E>: Sized {
    /// Add helpful context to errors
    ///
    /// Backtrace will be captured  if nightly feature is enabled
    ///
    /// `context` is provided as a closure to avoid potential formatting cost if
    /// the result isn't an error
    fn with_context<S: Into<String>, F: FnOnce() -> S>(self, context: F) -> Result<T, E>;
    /// Add helpful context to errors
    ///
    /// Backtrace will be captured  if nightly feature is enabled
    ///
    /// `context` is provided as a closure to avoid potential formatting cost if
    /// the result isn't an error
    fn context<S: Into<String>>(self, context: S) -> Result<T, E>;
}

#[macro_export]
macro_rules! ensure {
    ($cond:expr $(,)?) => {
        if !$cond {
            Err(anyhow::Error::msg(concat!("Condition failed: `", stringify!($cond), "`")))?;
        }
    };
    ($cond:expr, $msg:literal $(,)?) => {
        if !$cond {
            Err(anyhow::anyhow!($msg))?;
        }
    };
    ($cond:expr, $err:expr $(,)?) => {
        if !$cond {
            Err(anyhow::anyhow!($err))?;
        }
    };
    ($cond:expr, $fmt:expr, $($arg:tt)*) => {
        if !$cond {
            Err(anyhow::anyhow!($fmt, $($arg)*))?;
        }
    };
}

#[derive(thiserror::Error)]
pub enum Error {
    #[error("permission {0} denied")]
    PermissionDenied(Permission),

    #[error("anyhow::Error: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("std::fmt::Error: {0}")]
    StdFmt(#[from] std::fmt::Error),
    #[error("poise::serenity_prelude::Error: {0}")]
    Serenity(#[from] poise::serenity_prelude::Error),
    #[error("flume::SendError<String>: {0}")]
    FlumeSendString(#[from] flume::SendError<String>),
    #[error("poise::InvalidChoice: {0}")]
    InvalidChoice(#[from] poise::InvalidChoice),
    #[error("tokio::sync::oneshot::error::RecvError: {0}")]
    OneshotRecv(#[from] tokio::sync::oneshot::error::RecvError),
    #[error("flume::SendError<DbCommand>: {0}")]
    FlumeSendDbCommand(#[from] flume::SendError<DbCommand>),
    #[error("poise::serenity_prelude::ChannelIdParseError: {0}")]
    ChannelIdParse(#[from] poise::serenity_prelude::ChannelIdParseError),
    #[error("poise::serenity_prelude::RoleIdParseError: {0}")]
    RoleIdParse(#[from] poise::serenity_prelude::RoleIdParseError),
    #[error("std::num::ParseIntError: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("rusqlite::Error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("std::convert::Infallible: {0}")]
    Infallible(#[from] std::convert::Infallible),   
    #[error("std::error::Error: {0}")]
    StdErr(#[from] Box<dyn std::error::Error + std::marker::Send + Sync>),
    #[error("rusqlite_migration::Error: {0}")]
    RusqliteMigration(#[from] rusqlite_migration::Error),
    #[error("std::io::Error: {0}")]
    StdIo(#[from] std::io::Error),
    #[error("dotenv::Error: {0}")]
    DotEnv(#[from] dotenv::Error),
    #[error("tokio::task::JoinError: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("serde_json::Error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("chrono::OutOfRangeError: {0}")]
    ChronoOutOfRange(#[from] chrono::OutOfRangeError),
    #[error("mongodb::error::Error: {0}")]
    Mongo(#[from] mongodb::error::Error),
    
    #[cfg_attr(not(feature = "nightly"), error("WithContext [{0}]: {1}"))]
    #[cfg_attr(feature = "nightly", error("WithContext [{0}]: {1}\nBacktrace:\n{2}"))]
    WithContext(String, Box<Self>, #[cfg(feature = "nightly")] Option<Backtrace>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogBehaviour {
    pub log: bool,
    pub obfuscate: bool,
}

impl LogBehaviour {
    pub fn new(log: bool, obfuscate: bool) -> LogBehaviour {
        LogBehaviour { log, obfuscate }
    }
    pub fn user_only() -> LogBehaviour {
        LogBehaviour { log: false, obfuscate: false }
    }
    pub fn default() -> LogBehaviour {
        LogBehaviour { log: true, obfuscate: true }
    }
    pub fn user_safe() -> LogBehaviour {
        LogBehaviour { log: true, obfuscate: false }
    }
}

impl Error {
    pub fn log_behaviour(&self) -> LogBehaviour {
        match self {
            Error::PermissionDenied(_) => LogBehaviour::user_only(),
            Error::StdFmt(_) |
            Error::InvalidChoice(_) |
            Error::ChannelIdParse(_) |
            Error::RoleIdParse(_) |
            Error::ParseInt(_) => LogBehaviour::user_safe(),
            _ => LogBehaviour::default(),
        }
    }
}

// This makes sure thiserror messages get used on panic
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl<T, E: Into<Error>> ErrorContext<T, Error> for Result<T, E> {
    fn with_context<S: Into<String>, F: FnOnce() -> S>(self, context: F) -> Result<T, Error> {
        self.context(context())
    }
    fn context<S: Into<String>>(self, context: S) -> Result<T, Error> {
        self.map_err(|e| {
            Error::WithContext(
                context.into(),
                Box::new(e.into()),
                #[cfg(feature = "nightly")]
                Backtrace::capture(),
            )
        })
    }
}
