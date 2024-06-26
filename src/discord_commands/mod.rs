use poise::{self, Command};

use crate::{BotData, PoiseError};

mod testing;
use testing::*;

mod utils;
use utils::*;

mod config;
use config::*;

mod stats;
use stats::*;

mod permission;
use permission::*;

mod promote;
use promote::*;

mod purge;
use purge::*;

mod add_member;
use add_member::*;

pub fn commands() -> Vec<Command<BotData, PoiseError>> {
    vec![
        help(),
        ping(),
        get_config(),
        set_config(),
        delete_config(),
        check_config(),
        test_embed(),
        test_embed_success(),
        test_embed_error(),
        message_count(),
        test_greet_message(),
        set_log(),
        get_permissions(),
        grant_permission(),
        revoke_permission(),
        purge_permission(),
        get_table_sizes(),
        get_disk_space(),
        promote(),
        config_help(),
        purge(),
        add_member(),
        get_compression_state(),
    ]
}

pub fn chihuahua_commands() -> Vec<Command<BotData, PoiseError>> {
    vec![
        help(),
        ping(),
        get_config(),
        set_config(),
        delete_config(),
        check_config(),
        set_log(),
        get_permissions(),
        grant_permission(),
        revoke_permission(),
        purge_permission(),
        get_table_sizes(),
        get_disk_space(),
        config_help(),
    ]
}