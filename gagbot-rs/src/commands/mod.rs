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
mod permission;
pub use permission::*;
mod promote;
pub use promote::*;

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
        set_log(),
        get_permissions(),
        grant_permission(),
        revoke_permission(),
        purge_permission(),
        test_interaction_roles(),
        get_table_sizes(),
        get_disk_space(),
        promote(),
        config_help(),
        purge(),
    ]
}