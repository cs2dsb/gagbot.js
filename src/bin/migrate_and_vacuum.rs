use clap::Parser;
use tracing::*;

use gagbot_rs::{
    Error, 
    load_dotenv,
    configure_tracing, 
    db::{
        open_database,
        close_database, 
        vacuum_database,
    },
};

#[derive(Debug, Parser)]
#[clap(name = "gagbot.rs")]
struct Cli {
    #[clap(long, env, default_value = "gagbot.sqlite")]
    sqlite_connection_string: String,
}

// This simulates a single core vm: #[tokio::main(flavor = "multi_thread", worker_threads = 1)]
#[tokio::main]
async fn main() -> Result<(), Error> {
    load_dotenv()?;
    configure_tracing();

    let args = Cli::parse();
    debug!("Parsed args: {:#?}", args);

    let con = open_database(&args.sqlite_connection_string, true)?;
    
    vacuum_database(&con)?;
    close_database(con)?;
    Ok(())
}