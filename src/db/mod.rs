pub mod queries;
pub mod background_jobs;
mod db_command;
use std::{ffi::c_int, sync::Once, time::Duration};

pub use db_command::*;
use include_dir::{include_dir, Dir};
use rusqlite::{Connection, OpenFlags, TransactionBehavior};
use rusqlite_migration::Migrations;
use tokio::{sync::oneshot, task::JoinHandle, time::Instant};
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use crate::{db::queries::*, Error};

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

const MESSAGE_LOG_CHUNK_SIZE: u64 = 1024 * 100;
const COMPRESSION_LEVEL: i32 = zstd::DEFAULT_COMPRESSION_LEVEL;

pub fn get_migrations() -> Result<Migrations<'static>, Error> {
    Ok(Migrations::from_directory(&MIGRATIONS_DIR)?)
}

fn sqlite_tracing_callback(sqlite_code: c_int, msg: &str) {
    use rusqlite::ffi;
    let err_code = ffi::Error::new(sqlite_code);
    
    // See https://www.sqlite.org/rescode.html for description of result codes.
    match sqlite_code & 0xff {
        ffi::SQLITE_NOTICE => info!(target: "sqlite", msg, %err_code, "SQLITE NOTICE"),
        ffi::SQLITE_WARNING => warn!(target: "sqlite", msg, %err_code, "SQLITE WARNING"),
        _ => error!(target: "sqlite", msg, %err_code, "SQLITE ERROR"),
    };
}

fn sqlite_connection_profiling_callback(query: &str, duration: Duration) {
    trace!(target: "sqlite_profiling", ?duration, query);
}


#[instrument]
pub fn open_database(connection_string: &str, create: bool, run_migrations: bool) -> Result<Connection, Error> {
    // Configure the tracing callback before opening the database
    static CONFIG_LOG: Once = Once::new();
    let mut config_result = Ok(());
    CONFIG_LOG.call_once(|| {
        unsafe {
            config_result = rusqlite::trace::config_log(Some(sqlite_tracing_callback));
        }
    });
    config_result?;

    let mut open_flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;

    if create {
        open_flags |= OpenFlags::SQLITE_OPEN_CREATE;
    }

    let mut con = Connection::open_with_flags(connection_string, open_flags)?;
    con.profile(Some(sqlite_connection_profiling_callback));

    if run_migrations {
        let migrations = get_migrations()?;
        { 
            let _span = span!(Level::INFO, "Running migrations").entered();
            migrations.to_latest(&mut con)?;
        }
    }

    con.pragma_update(None, "journal_mode", "WAL")?;
    con.pragma_update(None, "synchronous", "NORMAL")?;
    con.pragma_update(None, "foreign_keys", "ON")?;

    debug!("Checking DB is writable");
    con.transaction_with_behavior(TransactionBehavior::Exclusive)?;

    Ok(con)
}

/// Runs an optimize on the database. Should be run periodically to keep the
/// database running optimally. It should be very fast if run regularly
#[instrument(skip(con))]
pub fn optimize_database(con: &Connection) -> Result<Duration, Error> {
    let start = Instant::now();
    con.pragma_update(None, "analysis_limit", "400")?;
    con.pragma_update(None, "optimize", "")?;

    Ok(start.elapsed())
}

#[instrument(skip(con))]
pub fn close_database(con: Connection) -> Result<(), Error> {
    optimize_database(&con)?;

    if let Err((_con, e)) = con.close() {
        Err(e)?;
    }

    Ok(())
}

// Vacuums the database to free up space and improve fragmentation
#[instrument(skip(con))]
pub fn vacuum_database(con: &Connection) -> Result<Duration, Error> {
    let start = Instant::now();
    con.execute("VACUUM", ())?;
    Ok(start.elapsed())
}

// Compress database contents (currently just the message log)
#[instrument(skip(con))]
pub fn compress_database(con: &mut Connection) -> Result<(Duration, bool), Error> {
    let start = Instant::now();
    let more = message_log::compress(con)?;
    Ok((start.elapsed(), more))
}

pub fn spawn_db_task(mut db_con: Connection, receiver: CommandReceiver) -> JoinHandle<Result<(), Error>> {
    fn respond<T, E>(respond_to: oneshot::Sender<Result<T, E>>, response: Result<T, E>, cmd_name: &str) -> Result<(), Error> {
        respond_to.send(response)
            .map_err(|_| anyhow::anyhow!("{cmd_name} respond_to oneshot closed"))?;
        Ok(())
    }

    tokio::task::spawn_blocking(move || {
        debug!("DB TASK: started");
        loop {
            match receiver.recv() {
                // The only error it returns is Disconnected (which we use to shut down)
                Err(_) => break,
                Ok(cmd) => {
                    let cmd_name = cmd.to_string();
                    let _span = span!(Level::INFO, "DB TASK", cmd = cmd_name).entered();
                    match cmd {
                        DbCommand::GetCompressionState { respond_to } => {
                            respond(respond_to, message_log::get_compression_state(&db_con), &cmd_name)?;
                        },
                        DbCommand::Optimize { respond_to } => {
                            respond(respond_to, optimize_database(&db_con), &cmd_name)?;
                        },
                        DbCommand::Vacuum { respond_to } => {
                            respond(respond_to, vacuum_database(&db_con), &cmd_name)?;
                        },
                        DbCommand::Compress { respond_to } => {
                            respond(respond_to, compress_database(&mut db_con), &cmd_name)?;
                        },
                        DbCommand::GetConfigString { guild_id, key, respond_to } => {
                            respond(respond_to, config::get(&db_con, guild_id, key), &cmd_name)?;
                        },
                        DbCommand::GetConfigI64 { guild_id, key, respond_to } => {
                            respond(respond_to, config::get(&db_con, guild_id, key), &cmd_name)?;
                        },
                        DbCommand::GetConfigU64 { guild_id, key, respond_to } => {
                            respond(respond_to, config::get(&db_con, guild_id, key), &cmd_name)?;
                        },
                        DbCommand::SetConfigString { guild_id, key, value, timestamp, respond_to } => {
                            respond(respond_to, config::update(&db_con, guild_id, key, &value, timestamp), &cmd_name)?;
                        },
                        DbCommand::DeleteConfig { guild_id, key, timestamp, respond_to } => {
                            respond(respond_to, config::delete(&db_con, guild_id, key, timestamp), &cmd_name)?;
                        },
                        DbCommand::GetLogChannel { guild_id, purpose, respond_to } => {
                            respond(respond_to, config::get_log_channel(&db_con, guild_id, &purpose), &cmd_name)?;
                        },
                        DbCommand::GetMessageCount { guild_id, user_id, channel_id, respond_to } => {
                            respond(respond_to, message_count::get(&db_con, guild_id, user_id, channel_id), &cmd_name)?;
                        },
                        DbCommand::IncrementMessageCount { guild_id, user_id, channel_id, respond_to } => {
                            respond(respond_to, message_count::increment(&db_con, guild_id, user_id, channel_id), &cmd_name)?;
                        },
                        DbCommand::GetGreet { guild_id, respond_to } => {
                            respond(respond_to, config::get_greet(&db_con, guild_id), &cmd_name)?;
                        },
                        DbCommand::GetMemberPermissions { guild_id, sorted_roles, respond_to } => {
                            respond(respond_to, permissions::get(&db_con, guild_id, sorted_roles), &cmd_name)?;
                        },
                        DbCommand::GrantPermission { guild_id, role_id, permission, respond_to, timestamp } => {
                            respond(respond_to, permissions::grant(&db_con, guild_id, role_id, permission, timestamp), &cmd_name)?;
                        },
                        DbCommand::RevokePermission { guild_id, role_id, permission, respond_to, timestamp } => {
                            respond(respond_to, permissions::grant(&db_con, guild_id, role_id, permission, timestamp), &cmd_name)?;
                        },
                        DbCommand::PurgePermissions { guild_id, respond_to, timestamp } => {
                            respond(respond_to, permissions::purge(&mut db_con, guild_id, timestamp), &cmd_name)?;
                        },
                        DbCommand::UpdateInteractionRoleSet { guild_id, name, description, channel_id, message_id, exclusive, timestamp, respond_to } => {
                            respond(respond_to, interaction_roles::update(&db_con, guild_id, name, description, channel_id, message_id, exclusive, timestamp), &cmd_name)?;
                        },
                        DbCommand::GetInteractionRole { guild_id, name, respond_to } => {
                            respond(respond_to, interaction_roles::get(&db_con, guild_id, name ), &cmd_name)?;
                        },
                        DbCommand::UpdateInteractionRoleChoice { guild_id, set_name, choice, emoji, role_id, timestamp, respond_to } => {
                            respond(respond_to, interaction_roles::update_choice(&db_con, guild_id, set_name, choice, emoji, role_id, timestamp ), &cmd_name)?;
                        },
                        DbCommand::LogMessage { message_id, timestamp, type_, message, respond_to } => {
                            respond(respond_to, message_log::log(&mut db_con, message_id, timestamp, type_, message), &cmd_name)?;
                        },
                        DbCommand::GetLogMessages { message_id, respond_to } => {
                            respond(respond_to, message_log::get(&db_con, message_id), &cmd_name)?;
                        },
                        // DbCommand::GetUserFromLogMessages{ guild_id, channel_id, message_id, respond_to } => {
                        //     respond(respond_to, message_log::get_user(&sqlite_con, guild_id, channel_id, message_id), &cmd_name)?;
                        // },
                        DbCommand::GetTableBytesAndCount { respond_to } => {
                            respond(respond_to, queries::get_table_size_in_bytes(&db_con), &cmd_name)?;
                        }
                    }
                },
            }
        }
        debug!("DB TASK: exiting");

        close_database(db_con)?;

        Ok::<_, Error>(())
    })
}