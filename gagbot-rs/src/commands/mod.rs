use poise::{self, Command};

use crate::{BotData, Error};

mod testing;
pub use testing::*;
mod utils;
pub use utils::*;
mod config;
pub use config::*;
mod stats;
pub use stats::*;

pub fn commands() -> Vec<Command<BotData, Error>> {
    vec![
        help(),
        ping(),
        get_config(),
        set_config(),
        delete_config(),
        test_embed(),
        test_embed_success(),
        test_embed_error(),
        message_count(),
        test_greet(),
    ]
}
