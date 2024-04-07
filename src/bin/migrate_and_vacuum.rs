use std::{num::NonZeroUsize, time::Duration};

use clap::Parser;
use rusqlite_migration::SchemaVersion;
use tokio::time::sleep;
use tracing::*;

use gagbot_rs::{
    configure_tracing, db::{
        close_database, get_migrations, open_database, vacuum_database
    }, load_dotenv, Error
};

#[derive(Debug, Parser)]
#[clap(name = "gagbot.rs")]
struct Cli {
    #[clap(long, env, default_value = "gagbot.sqlite")]
    sqlite_connection_string: String,
    #[clap(long, default_value = "false")]
    vacuum: bool,
    #[clap(long, default_value = "5")]
    bulk_operation_sleep_seconds: u64,
}

// This simulates a single core vm: #[tokio::main(flavor = "multi_thread", worker_threads = 1)]
#[tokio::main]
async fn main() -> Result<(), Error> {
    load_dotenv()?;
    configure_tracing();

    let args = Cli::parse();
    debug!("Parsed args: {:#?}", args);

    let mut con = open_database(&args.sqlite_connection_string, true, false)?;

    let migrations = get_migrations()?;
    
    loop {
        let current_version = migrations.current_version(&con)?;
        info!("Migration version: {}", current_version);
        
        if let SchemaVersion::Inside(n) = current_version {
            let eight = NonZeroUsize::new(8).unwrap();
            
            let target_version = if n < eight {
                // We haven't started rekeying message_log
                Some(8)
            } else if n == eight {
                // We are mid run
                info!("Rekeying...");

                let _span = span!(Level::INFO, "Rekeying messages").entered();
                let from_count = con
                    .prepare_cached("SELECT count(*) FROM message_log")?
                    .query_row((), |r| r.get::<_, usize>(0))?;
                let to_count = con
                    .prepare_cached("SELECT count(*) FROM message_log_with_id")?
                    .query_row((), |r| r.get::<_, usize>(0))?;



                // Some(usize::MAX)
                None
            } else {
                Some(usize::MAX)
            };

            if let Some(version) = target_version {
                migrations.to_version(&mut con, version)?;
                if version == usize::MAX {
                    break;
                }
            }
        } else {
            error!("Current version isn't 'Inside' known migrations");
            break;
        }

        sleep(Duration::from_secs(args.bulk_operation_sleep_seconds)).await;
    }

    if args.vacuum {
        info!("Vacuuming...");
        vacuum_database(&con)?;
    }

    close_database(con)?;
    info!("Done");
    Ok(())
}