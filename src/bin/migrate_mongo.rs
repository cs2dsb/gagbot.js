use std::collections::HashMap;

use clap::Parser;
use dotenv::dotenv;
use futures::TryStreamExt;
use gagbot_rs::{db::queries::config::*, *};
use mongodb::{
    bson::{doc, oid::ObjectId, Bson as BsonValue, Document},
    options::{ClientOptions, ServerApi, ServerApiVersion},
    Client,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, error};
#[derive(Debug, Parser)]
#[clap(name = "gagbot.rs")]
struct Cli {
    #[clap(long, env)]
    mongo_db_uri: String,
    #[clap(long, env)]
    mongo_db_database: String,
    #[clap(long, env, default_value = "gagbot.sqlite")]
    sqlite_connection_string: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Permissions {
    pub roles: HashMap<String, HashMap<String, bool>>,

    #[serde(flatten, skip_serializing)]
    pub unrecognized_fields: HashMap<String, BsonValue>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Greet {
    pub message: Option<String>,
    pub role: Option<String>,
    pub channel: Option<String>,
    pub welcomechannel: Option<String>,
    pub welcomemessage: Option<String>,

    #[serde(flatten, skip_serializing)]
    pub unrecognized_fields: HashMap<String, BsonValue>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PromoteRules {
    pub new_chat_channel: Option<String>,
    pub junior_chat_channel: Option<String>,
    pub new_chat_min_messages: Option<i32>,
    pub junior_chat_min_messages: Option<i32>,
    pub junior_min_age: Option<i32>,
    pub new_message_max_age: Option<i32>,

    #[serde(flatten, skip_serializing)]
    pub unrecognized_fields: HashMap<String, BsonValue>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PromoteRoles {
    pub junior_role: Option<String>,
    pub full_role: Option<String>,

    #[serde(flatten, skip_serializing)]
    pub unrecognized_fields: HashMap<String, BsonValue>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Data {
    pub greet: Option<Greet>,
    #[serde(rename = "promoterules")]
    pub promote_rules: Option<PromoteRules>,
    #[serde(rename = "promoteroles")]
    pub promote_roles: Option<PromoteRoles>,

    #[serde(flatten, skip_serializing)]
    pub unrecognized_fields: HashMap<String, BsonValue>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Guild {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub permissions: Permissions,
    pub data: Data,

    pub _id: ObjectId,
    pub __v: i32,

    #[serde(flatten, skip_serializing)]
    pub unrecognized_fields: HashMap<String, BsonValue>,
}

impl Guild {
    fn collection() -> &'static str {
        "guilds"
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LogChannels {
    #[serde(rename = "guild")]
    pub guild_id: String,
    pub channel: String,
    #[serde(rename = "logTypes")]
    pub log_types: Vec<String>,

    pub _id: ObjectId,
    pub __v: i32,

    #[serde(flatten, skip_serializing)]
    pub unrecognized_fields: HashMap<String, BsonValue>,
}

impl LogChannels {
    fn collection() -> &'static str {
        "logchannels"
    }
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RoleSets {
    #[serde(rename = "guild")]
    pub guild_id: String,
    pub exclusive: bool,
    #[serde(rename = "alias")]
    pub name: String,
    #[serde(rename = "channel")]
    pub channel_id: Option<String>,
    #[serde(rename = "message")]
    pub message_id: Option<String>,
    #[serde(default)]
    pub choices: HashMap<String, String>,

    pub _id: ObjectId,
    pub __v: i32,

    #[serde(flatten, skip_serializing)]
    pub unrecognized_fields: HashMap<String, BsonValue>,
}

impl RoleSets {
    fn collection() -> &'static str {
        "rolesets"
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv()?;

    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_line_number(true)
            .with_file(true)
            .finish(),
    )
    .expect("Failed to set default tracing subscriber");

    let args = Cli::parse();
    debug!("Parsed args: {:#?}", args);

    let mut sqlite_con = open_database(&args.sqlite_connection_string, true)?;

    let mongo_client = {
        let mut client_options = ClientOptions::parse(args.mongo_db_uri).await?;
        // Set the server_api field of the client_options object to Stable API version 1
        client_options.server_api =
            Some(ServerApi::builder().version(ServerApiVersion::V1).build());

        let client = Client::with_options(client_options)?;
        // Make sure we're correctly connected before trying discord
        debug!("pinging mongodb: start");
        client
            .database("admin")
            .run_command(doc! {"ping": 1}, None)
            .await?;
        debug!("pinging mongodb: done");

        client
    };

    let mongo_db = mongo_client.database(&args.mongo_db_database);

    let guilds: Vec<Guild> = match mongo_db
        .collection(Guild::collection())
        .find(None, None)
        .await?
        .try_collect()        
        .await
        .context("Fetching and parsing Guild::collection()")
    {
        Ok(r) => Ok(r),
        Err(e) => {
            let docs: Result<Vec<Document>, _> = mongo_db
                .collection(Guild::collection())
                .find(None, None)
                .await?
                .try_collect()        
                .await
                .context("Fetching Guild::collection()");

            if let Ok(docs) = docs {
                error!("Document that failed parsing: {:#?}", docs);
            }
            Err(e)
        },
    }?;

    let log_channels = {
        let rows: Vec<LogChannels> = mongo_db
            .collection(LogChannels::collection())
            .find(None, None)
            .await?
            .try_collect()
            .await?;

        let mut ret: Vec<LogChannels> = Vec::new();
        for mut v in rows.into_iter() {
            for existing in ret.iter_mut().filter(|x| x.guild_id == v.guild_id) {
                // This is lossy but it's following the same logic the existing js version uses
                // (first result takes precidence)
                for t in v.log_types.clone().into_iter() {
                    if existing.log_types.contains(&t) {
                        v.log_types.retain(|t_| t_ != &t);
                    }
                }
            }

            if v.log_types.len() > 0 {
                ret.push(v);
            }
        }
        ret
    };

    let role_sets: Vec<RoleSets> = mongo_db
        .collection(RoleSets::collection())
        .find(None, None)
        .await?
        .try_collect()
        .await?;

    for guild in guilds.iter() {
        let tx = sqlite_con.transaction()?;
        let guild_id = &guild.id;

        if tx
            .prepare_cached("SELECT 1 FROM guild WHERE id = ?1 LIMIT 1")?
            .exists(&[guild_id])?
        {
            continue;
        }

        debug!("Migrating guild {} ({})", guild.name, guild.id);

        {
            let mut permission_stmt = tx.prepare_cached(
                "INSERT INTO permission (guild_id, discord_id, type, value)
            VALUES (?1, ?2, ?3, ?4);",
            )?;
            for (role_id, permissions) in guild.permissions.roles.iter() {
                for (value, _) in permissions.iter().filter(|(_, v)| **v) {
                    permission_stmt.execute(&[guild_id, role_id, "ROLE", value])?;
                }
            }
        }

        if let Some(greet) = &guild.data.greet {
            let mut config_stmt = tx.prepare_cached(
                "INSERT INTO config (guild_id, key, value)
            VALUES (?1, ?2, ?3);",
            )?;

            if let Some(message) = greet.message.as_ref() {
                config_stmt.execute(params![guild_id, ConfigKey::GreetMessage, message])?;
            }
            if let Some(channel) = greet.channel.as_ref() {
                config_stmt.execute(params![guild_id, ConfigKey::GreetChannel, channel])?;
            }
            if let Some(role) = greet.role.as_ref() {
                config_stmt.execute(params![guild_id, ConfigKey::GreetRole, role])?;
            }
            if let Some(welcomemessage) = greet.welcomemessage.as_ref() {
                config_stmt.execute(params![guild_id, ConfigKey::GreetWelcomeMessage, welcomemessage])?;
            }
            if let Some(welcomechannel) = greet.welcomechannel.as_ref() {
                config_stmt.execute(params![guild_id, ConfigKey::GreetWelcomeChannel, welcomechannel])?;
            }
        }

        if guild.data.promote_roles.is_some() || guild.data.promote_rules.is_some() {
            let new_chat_channel = guild
                .data
                .promote_rules
                .as_ref()
                .map(|v| v.new_chat_channel.to_owned())
                .flatten();
            let junior_chat_channel = guild
                .data
                .promote_rules
                .as_ref()
                .map(|v| v.junior_chat_channel.to_owned())
                .flatten();
            let new_chat_min_messages = guild
                .data
                .promote_rules
                .as_ref()
                .map(|v| v.new_chat_min_messages.map(|v| format!("{}", v)))
                .flatten();
            let junior_chat_min_messages = guild
                .data
                .promote_rules
                .as_ref()
                .map(|v| v.junior_chat_min_messages.map(|v| format!("{}", v)))
                .flatten();
            let junior_min_age = guild
                .data
                .promote_rules
                .as_ref()
                .map(|v| v.junior_min_age.map(|v| format!("{}", v)))
                .flatten();
            let junior_role = guild
                .data
                .promote_roles
                .as_ref()
                .map(|v| v.junior_role.to_owned())
                .flatten();
            let full_role = guild
                .data
                .promote_roles
                .as_ref()
                .map(|v| v.full_role.to_owned())
                .flatten();

            let config = vec![
                (ConfigKey::PromoteNewChatChannel, new_chat_channel),
                (ConfigKey::PromoteJuniorChatChannel, junior_chat_channel),
                (ConfigKey::PromoteJuniorRole, junior_role),
                (ConfigKey::PromoteFullRole, full_role),
                (ConfigKey::PromoteNewChatMinMessages, new_chat_min_messages),
                (
                    ConfigKey::PromoteJuniorChatMinMessages,
                    junior_chat_min_messages,
                ),
                (ConfigKey::PromoteJuniorMinAge, junior_min_age),
            ];

            let mut config_stmt = tx.prepare_cached(
                "INSERT INTO config (guild_id, key, value) 
            VALUES (?1, ?2, ?3);",
            )?;
            for (key, value) in config.into_iter().filter(|(_, v)| v.is_some()) {
                config_stmt.execute(params![guild_id, key, &value.unwrap()])?;
            }
        }

        for log_channel in log_channels
            .iter()
            .filter(|v| v.guild_id.as_str() == guild_id)
        {
            let mut config_stmt = tx.prepare_cached(
                "INSERT INTO config (guild_id, key, value)            
            VALUES (?1, ?2, ?3);",
            )?;

            for lt in log_channel.log_types.iter() {
                for key in match lt.as_str() {
                    "message" => vec![ConfigKey::LoggingEditsAndDeletes],
                    "member" => vec![ConfigKey::LoggingJoiningAndLeaving],
                    "error" => vec![ConfigKey::LoggingErrors, ConfigKey::LoggingGeneral],
                    "voice" => vec![ConfigKey::LoggingVoiceActivity],
                    _ => vec![],
                }
                .iter()
                {
                    config_stmt.execute(params![guild_id, *key, &log_channel.channel])?;
                }
            }
        }

        for role_set in role_sets
            .iter()
            .filter(|rs| rs.channel_id.is_some() && rs.message_id.is_some())
        {
            let mut set_stmt = tx.prepare_cached(
                "INSERT INTO reaction_role_temp 
            (guild_id, exclusive, name, channel_id, message_id)
            VALUES (?1, ?2, ?3, ?4, ?5);",
            )?;

            let mut choice_stmt = tx.prepare_cached(
                "INSERT INTO reaction_role_choice_temp 
            (guild_id, set_name, choice, role_id)
            VALUES (?1, ?2, ?3, ?4);",
            )?;

            set_stmt.execute(params![
                guild_id,
                if role_set.exclusive { 1 } else { 0 },
                role_set.name,
                role_set.channel_id,
                role_set.message_id,
            ])?;

            for (choice, role_id) in role_set.choices.iter() {
                choice_stmt.execute(params![guild_id, role_set.name, choice, role_id,])?;
            }
        }

        {
            let mut guild_stmt =
                tx.prepare_cached("INSERT INTO guild (id, name) VALUES (?1, ?2)")?;
            guild_stmt.execute(&[guild_id, &guild.name])?;
        }

        tx.commit()?;

        info!("Migrated guild {} ({})", guild.name, guild.id);
    }

    close_database(sqlite_con)?;

    Ok(())
}
