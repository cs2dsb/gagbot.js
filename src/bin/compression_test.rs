#![allow(unused_imports, dead_code)]
use std::{io::Cursor, num::ParseIntError, path::Path, time::Duration};

use clap::Parser;
use futures::future::{select, Either};
use humansize::{make_format, BINARY};
use temp_dir::TempDir;
use tracing::*;

use gagbot_rs::{
    configure_tracing, db::{
        background_jobs::spawn_db_background_jobs_task, close_database, open_database, queries::message_log::{self, compress, verify_compressed_chunks, LogType}, spawn_db_task, vacuum_database, DbCommand 
    }, load_dotenv, Error
};
use zstd::encode_all;

// 0 means use the default (3)
const COMPRESSION_LEVEL: i32 = 0;

fn frequency_seconds_valid_range(s: &str) -> Result<u64, String> {
    let v = s.parse().map_err(|e: ParseIntError| e.to_string())?;

    if v < 60 {
        Err(format!(
            "Running more often than once per minute ({} seconds) is not recommended",
            v
        ))?;
    }
    Ok(v)
}

#[derive(Debug, Parser)]
#[clap(name = "gagbot.rs")]
struct Cli {
    #[clap(long, env, default_value = "gagbot.sqlite")]
    sqlite_connection_string: String,
    #[clap(long, env, default_value = "64")]
    database_command_channel_bound: usize,
    #[clap(long, env, default_value = "3600", value_parser = frequency_seconds_valid_range)]
    background_task_frequency_seconds: u64,
}

// This simulates a single core vm: #[tokio::main(flavor = "multi_thread", worker_threads = 1)]
#[tokio::main]
async fn main() -> Result<(), Error> {
    load_dotenv()?;
    configure_tracing();

    let args = Cli::parse();
    debug!("Parsed args: {:#?}", args);

    let mut sqlite_con = open_database(&args.sqlite_connection_string, true)?;
    
    let mut did_work = false;
    while compress(&mut sqlite_con)? {
        did_work = true;
    }

    {
        let _span = span!(Level::INFO, "Verifying compressed chunks").entered();
        verify_compressed_chunks(&sqlite_con)?;
    }
    // (message_id, log entry count, log entry with payload count)
    let test_data = [
        (1222229448384839702, 1, 1), // Chunk temp
        (1209103072832266270_u64, 8_usize, 8), // 1 chunk
        (1122243681722826853, 13, 13), // 2 chunks
        (1166501709405769798, 6, 6), // 3 chunks
        (1222104462550630453, 2, 1), // 1 chunk
    ];

    {
        let _span = span!(Level::INFO, "Verifying message logs").entered();
        for (id, count, count_body) in test_data.into_iter() {
            let log = message_log::get(&sqlite_con, id.into())?;
            if log.len() != count {
                error!(found=log.len(), expected=count, message_id=id, "Got the wrong number of log entries for message")
            }

            let actual_count = log
                .iter()
                .filter(|m| m.message.is_some()) 
                .count();
            if actual_count != count_body {
                error!(count_body, actual_count, "Unexpected message body count for message_id {id}");
            }
        }
    }

    if did_work {
        vacuum_database(&sqlite_con)?;
    }
    close_database(sqlite_con)?;

    Ok(())
}