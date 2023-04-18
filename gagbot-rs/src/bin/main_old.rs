// use dotenv::dotenv;
// use clap::Parser;
// use mongodb::{
// bson::{
// doc,
// Bson as BsonValue,
// oid::ObjectId,
// },
// Client,
// Collection,
// options::{
// ClientOptions,
// ServerApi,
// ServerApiVersion,
// ReplaceOptions,
// },
// Database,
// };
// use futures::stream::StreamExt;
// use std::collections::{ HashMap, HashSet };
// use serde::{ Serialize, Deserialize };
// use poise::{ self, serenity_prelude as serenity };
// use serenity::{ GuildId, guild::PartialGuild };
// use tracing::{ debug };
// use gagbot_rs::*;
//
// #[derive(Debug, Parser)]
// #[clap(name = "gagbot.rs")]
// struct Cli {
// #[clap(long, env)]
// discord_token: String,
// #[clap(long, env)]
// mongo_db_uri: String,
// #[clap(long, env)]
// mongo_db_database: String,
// }
//
// #[derive(Debug, Default, Serialize, Deserialize)]
// pub struct Permissions {
// pub roles: HashMap<String, HashMap<String, bool>>,
//
// #[serde(flatten, skip_serializing)]
// pub unrecognized_fields: HashMap<String, BsonValue>,
// }
//
// #[derive(Debug, Default, Serialize, Deserialize)]
// pub struct Greet {
// pub message: String,
// pub role: String,
// pub channel: String,
// pub welcomechannel: String,
// pub welcomemessage: String,
//
// #[serde(flatten, skip_serializing)]
// pub unrecognized_fields: HashMap<String, BsonValue>,
// }
//
// #[derive(Debug, Default, Serialize, Deserialize)]
// pub struct PromoteRules {
// pub new_chat_channel: String,
// pub junior_chat_channel: String,
// pub new_chat_min_messages: i32,
// pub junior_chat_min_messages: i32,
// pub junior_min_age: i32,
//
// #[serde(flatten, skip_serializing)]
// pub unrecognized_fields: HashMap<String, BsonValue>,
// }
//
// #[derive(Debug, Default, Serialize, Deserialize)]
// pub struct PromoteRoles {
// pub junior_role: String,
// pub full_role: String,
//
// #[serde(flatten, skip_serializing)]
// pub unrecognized_fields: HashMap<String, BsonValue>,
// }
//
// #[derive(Debug, Default, Serialize, Deserialize)]
// pub struct Data {
// pub greet: Greet,
// #[serde(rename = "promoterules")]
// pub promote_rules: PromoteRules,
// #[serde(rename = "promoteroles")]
// pub promote_roles: PromoteRoles,
//
// #[serde(flatten, skip_serializing)]
// pub unrecognized_fields: HashMap<String, BsonValue>,
// }
//
// #[derive(Debug, Default, Serialize, Deserialize)]
// pub struct Guild {
// pub id: String,
// pub name: String,
// pub prefix: String,
// pub permissions: Permissions,
// pub data: Data,
//
// pub _id: ObjectId,
// pub __v: i32,
//
// #[serde(flatten, skip_serializing)]
// pub unrecognized_fields: HashMap<String, BsonValue>,
// }
//
// impl Guild {
// fn collection() -> &'static str {
// "guilds"
// }
//
// fn new(id: String, name: String) -> Self {
// Self {
// id,
// name,
// .. Default::default()
// }
// }
// }
//
// Data passed to all bot commands by the framework
// #[derive(Debug)]
// struct BotData {
// db: Database,
// }
//
// impl BotData {
// fn new(db: Database) -> Self {
// Self {
// db,
// }
// }
//
// async fn get_guild(&self, guild: PartialGuild) -> anyhow::Result<Guild> {
// let collection = self.db.collection(Guild::collection());
// let id_string = guild.id.to_string();
//
// Ok(collection
// .find_one(doc! { "id": &id_string }, None)
// .await?
// .unwrap_or(Guild::new(id_string, guild.name.to_string())))
// }
//
// async fn save_guild(&self, guild: &Guild) -> anyhow::Result<()> {
// let collection: Collection<Guild> = self.db.collection(Guild::collection());
// let id_string = guild.id.clone();
//
// collection.replace_one(
// doc! { "id": &id_string },
// guild,
// Some(ReplaceOptions::builder()
// .upsert(true)
// .build()),
// ).await?;
//
// Ok(())
// }
// }
// type Error = Box<dyn std::error::Error + Send + Sync>;
// type Context<'a> = poise::Context<'a, BotData, Error>;
//
// #[poise::command(prefix_command, slash_command)]
// async fn help(
// ctx: Context<'_>,
// #[description = "Command to display specific information about"] command:
// Option<String>, ) -> Result<(), Error> {
// poise::builtins::help(ctx, command.as_deref(), Default::default()).await?;
// Ok(())
// }
//
// #[poise::command(
// prefix_command,
// slash_command,
// category = "General"
// )]
// Ping pong
// async fn ping(ctx: Context<'_>) -> Result<(), Error> {
// ctx.say("Pong! : )").await?;
//
// Ok(())
// }
//
// Displays your or another user's account creation date
// #[poise::command(prefix_command, slash_command)]
// async fn age(
// ctx: Context<'_>,
// #[description = "Selected user"] user: Option<serenity::User>,
// ) -> Result<(), Error> {
// let u = user.as_ref().unwrap_or_else(|| ctx.author());
// let response = format!("{}'s account was created at {}", u.name,
// u.created_at()); ctx.say(response).await?;
// Ok(())
// }
//
// Displays the ID of the current guild
// #[poise::command(prefix_command, slash_command, guild_only)]
// async fn guild_id(
// ctx: Context<'_>,
// ) -> Result<(), Error> {
// let guild = ctx.guild_id()
// .expect("missing guild in 'guild_only' command");
//
// let response = format!("Guild ID: {:?}", guild);
// ctx.say(response).await?;
//
// Ok(())
// }
//
// Changes the prefix in the current guild
// #[poise::command(prefix_command, slash_command, guild_only)]
// #[tracing::instrument]
// async fn set_prefix(
// ctx: Context<'_>,
// #[description = "The new prefix"] new_prefix: String,
// ) -> Result<(), Error> {
// let guild = ctx.partial_guild()
// .await
// .expect("missing guild in 'guild_only' command");
//
// let mut guild = ctx.data().get_guild(guild).await?;
//
// let response = format!("Prefix changed from '{}' to '{}'", guild.prefix,
// new_prefix);
//
// guild.prefix = new_prefix;
// ctx.data().save_guild(&guild).await?;
//
// ctx.say(response).await?;
//
// Ok(())
// }
//
// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
// dotenv()?;
//
// tracing::subscriber::set_global_default(
// tracing_subscriber::FmtSubscriber::builder()
// .with_max_level(tracing::Level::TRACE)
// .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
// .with_line_number(true)
// .with_file(true)
// .finish(),
// )
// .expect("Failed to set default tracing subscriber");
//
// let args = Cli::parse();
//
// let mut client_options = ClientOptions::parse(args.mongo_db_uri).await?;
// Set the server_api field of the client_options object to Stable API version 1
// client_options.server_api =
// Some(ServerApi::builder().version(ServerApiVersion::V1).build());
//
// let client = Client::with_options(client_options)?;
// let db = client.database(&args.mongo_db_database);
// Make sure we're correctly connected before trying discord
// debug!("pinging mongodb: start");
// db.run_command(doc! {"ping": 1}, None).await?;
// debug!("pinging mongodb: done");
//
// let framework = poise::Framework::builder()
// .options(poise::FrameworkOptions {
// commands: vec![
// help(),
// ping(),
// age(),
// guild_id(),
// set_prefix(),
// ],
// on_error: |err| Box::pin(poise_error_handler(err)),
// .. Default::default()
// })
// .token(args.discord_token)
// TODO: are all needed?
// .intents(serenity::GatewayIntents::all())
// .setup(|ctx, ready, framework| {
// debug!("Discord connected");
// Box::pin(async move {
// poise::builtins::register_globally(
//     ctx,
//     &framework.options().commands,
// ).await?;
//
// This clears global commands
// serenity::Command::set_global_application_commands(
//     ctx,
//     |b| b,
// ).await?;
//
// for g in ready.guilds.iter() {
// poise::builtins::register_in_guild(
// ctx,
// &framework.options().commands,
// g.id,
// ).await?;
// }
//
// Ok(BotData::new(db))
// })
// });
//
// framework.run().await?;
//
// Ok(())
// }
//
// async fn poise_error_handler(error: poise::FrameworkError<'_, BotData,
// Error>) { println!("Oh noes, we got an error: {:?}", error);
// }

fn main() {
    unimplemented!()
}
