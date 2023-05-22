use std::{path::PathBuf, time::Duration, fmt::{Debug, self}};

use anyhow::{Result, Context as AnyhowContext};
use chrono::{Utc, Days};
use config::{ConfigKey, LogChannel};
use include_dir::{include_dir, Dir};
use interaction_roles::InteractionRole;
use lazy_regex::{regex, Captures};
use message_log::{LogType, MessageLog};
use permissions::{EffectivePermission, Permission};
use poise::serenity_prelude::{Guild, Member, Timestamp, User, Http, Channel, ChannelType, CacheHttp, Cache, Message};
use rusqlite::{Connection, OpenFlags, TransactionBehavior};
use rusqlite_migration::Migrations;
use tokio::sync::oneshot;
use std::str::FromStr;

pub mod config;
pub mod message_count;

mod ids;
pub use ids::*;

mod db;
pub use db::*;

pub mod permissions;
pub mod interaction_roles;
pub mod message_log;

mod embed;
pub use embed::*;
use tracing::{debug };

pub mod commands;

pub const GAGBOT_ICON: &str = "https://cdn.discordapp.com/emojis/708352151558029322.png";
pub const GAGBOT_ICON_ERROR: &str = "https://cdn.discordapp.com/emojis/708352247804854285.png";

pub const GAGBOT_COLOR_NORMAL: i32 = 0xEBC634;
pub const GAGBOT_COLOR_ERROR: i32 = 0xFF0000;
pub const GAGBOT_COLOR_SUCCESS: i32 = 0x00FF00;
pub const GAGBOT_COLOR_LOG_EDIT: i32 = 0x30649c;
pub const GAGBOT_COLOR_LOG_DELETE: i32 = 0x9c3730;
pub const GAGBOT_COLOR_GREET: i32 = 0x65e7b7;
pub const GAGBOT_COLOR_LOG_JOIN: i32 = 0x009900;
pub const GAGBOT_COLOR_LOG_LEAVE: i32 = 0x990044;

pub const INTERACTION_BUTTON_CUSTOM_ID_MAX_LEN: usize = 100;
pub const INTERACTION_BUTTON_CUSTOM_ID_ROLE_ID_MAX_LEN: usize = 21;
pub const INTERACTION_BUTTON_CUSTOM_ID_DELIMITER: char = '¬';
pub const INTERACTION_BUTTON_CUSTOM_ID_PREFIX: &str = "rr";
pub const INTERACTION_BUTTON_CUSTOM_ID_NAME_MAX_LEN: usize = 
    INTERACTION_BUTTON_CUSTOM_ID_MAX_LEN 
    - INTERACTION_BUTTON_CUSTOM_ID_ROLE_ID_MAX_LEN
    - 1 //delimiter
    - INTERACTION_BUTTON_CUSTOM_ID_PREFIX.len();

/// The edit tracking functionality won't work without some cached messages
/// 200 is the default from discord.js <https://github.com/discordjs/discord.js/blob/86e5f5a119c6d2588b988a33236d358ded357847/packages/discord.js/src/util/Options.js#L175>
pub const CACHE_MAX_MESSAGES: usize = 200;

pub const DISK_SPACE_WARNING_LEVEL: u64 = 5*1024*1024*1024;

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

#[derive(Debug, Clone)]
pub struct BotData {
    pub db_command_sender: flume::Sender<DbCommand>,
    pub db_file_path: Option<PathBuf>,
    pub background_task_frequency: Duration,
}

impl BotData {
    pub fn new(
        db_command_sender: flume::Sender<DbCommand>, 
        db_file_path: Option<PathBuf>,
        background_task_frequency: Duration,
    ) -> Self {
        Self {
            db_command_sender,
            db_file_path,
            background_task_frequency,
        }
    }

    pub fn db_available_space(&self) -> anyhow::Result<u64> {
        if self.db_file_path.is_none() {
            anyhow::bail!("DB appears to not be disk backed? Can't check the available space");
        }

        Ok(fs2::available_space(self.db_file_path.as_ref().unwrap())?)
    }

    pub async fn general_log_channel(&self, guild_id: GuildId) -> Result<Option<ChannelId>> {
        self.log_channel(guild_id, vec![config::LogChannel::General])
            .await
    }

    pub async fn error_log_channel(&self, guild_id: GuildId) -> Result<Option<ChannelId>> {
        self.log_channel(
            guild_id,
            vec![config::LogChannel::Error, config::LogChannel::General],
        )
        .await
    }

    pub async fn log_channel(
        &self,
        guild_id: GuildId,
        purpose: Vec<LogChannel>,
    ) -> Result<Option<ChannelId>> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetLogChannel {
                guild_id,
                purpose,
                respond_to: s,
            })
            .await?;
        Ok(r.await??.map(|v| v.into()))
    }

    pub async fn message_count(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        channel_id: Option<ChannelId>,
    ) -> Result<usize> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetMessageCount {
                guild_id,
                user_id,
                channel_id: channel_id,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn get_greet(
        &self,
        guild_id: GuildId,
        user: &User,
    ) -> Result<Option<(ChannelId, Embed)>> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetGreet {
                guild_id,
                respond_to: s,
            })
            .await?;

        if let Some((channel_id, message)) = r.await?? {
            let replace_regex = regex!(r"\{\{([^{}]+)}}");
            let user = std::sync::Arc::new(user);
            let message = replace_regex.replace_all(&message, |caps: &Captures| match &caps[0] {
                "{{tag}}" => user.to_string(),
                "{{name}}" => user.name.clone(),
                "{{discriminator}}" => user.discriminator.to_string(),
                _ => format!("{{{{ unknown replacement \"{}\" }}", &caps[0]),
            });
            let message = message.replace("\\n", "\n").to_string();

            let mut embed = Embed::default()
                .description(message)
                .color(GAGBOT_COLOR_GREET);
            embed.thumbnail_url = user.avatar_url();

            Ok(Some((channel_id, embed)))
        } else {
            Ok(None)
        }
    }

    pub async fn increment_message_count(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<()> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::IncrementMessageCount {
                guild_id,
                user_id,
                channel_id,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn set_config(
        &self,
        guild_id: GuildId,
        key: ConfigKey,
        timestamp: Timestamp,
        value: String,
    ) -> Result<()> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::SetConfigString {
                guild_id,
                key,
                value,
                timestamp,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn get_member_permissions(
        &self,
        guild: &Guild,
        member: &Member,
    ) -> Result<Vec<EffectivePermission>> {
        let sorted_roles = {
            let mut roles = guild.roles
            .values()            
            // Filter the roles down to only the ones the member has
            .filter_map(|v| if member.roles.contains(&v.id) {
                Some((v.position, v.id))
            } else {
                None
            })
            .collect::<Vec<_>>();
            
            // Sort by position so higher roles are on the top
            roles.sort_by(|(a, _), (b, _)| b.cmp(a));

            roles.into_iter().map(|(_, b)| b.into()).collect::<Vec<_>>()
        };
            
        let guild_id = member.guild_id.into();
             
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetMemberPermissions {
                guild_id,
                sorted_roles,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }   

    pub async fn require_permission(
        &self,
        guild: &Guild,
        member: &Member,
        permission: Permission,
    ) -> Result<EffectivePermission> {
        let effective_permission = self
            .get_member_permissions(guild, member).await?
            .into_iter()
            // TODO: do we need anything more sophisticated like a tree of permissions?
            .find(|x| x.permission == permission || x.permission == Permission::All);

        effective_permission.ok_or(anyhow::anyhow!("Permission denied"))
    }   
    
    pub async fn grant_permission(
        &self,
        guild_id: GuildId,
        role_id: RoleId,
        permission: Permission,
        timestamp: Timestamp,
    ) -> Result<bool> {             
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GrantPermission {
                guild_id,
                role_id,
                permission,
                timestamp,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }  
    
    pub async fn revoke_permission(
        &self,
        guild_id: GuildId,
        role_id: RoleId,
        permission: Permission,
        timestamp: Timestamp,
    ) -> Result<bool> {             
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::RevokePermission {
                guild_id,
                role_id,
                permission,
                timestamp,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }  
    
    pub async fn purge_permission(
        &self,
        guild_id: GuildId,
        timestamp: Timestamp,
    ) -> Result<bool> {             
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::PurgePermissions {
                guild_id,
                timestamp,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }  
    
    pub async fn get_interaction_role(
        &self,
        guild_id: GuildId,
        name: String,
    ) -> Result<Option<InteractionRole>> {             
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetInteractionRole {
                guild_id,
                name,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }  
    
    pub async fn update_interaction_role(
        &self,
        guild_id: GuildId,
        name: String,
        description: Option<String>,
        channel_id: ChannelId,
        message_id: Option<MessageId>,
        exclusive: bool,
        timestamp: Timestamp,
    ) -> Result<bool> {             
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::UpdateInteractionRoleSet {
                guild_id,
                name,
                description,
                channel_id,
                message_id,
                exclusive,
                timestamp,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    } 
    
    pub async fn update_interaction_choice(
        &self,
        guild_id: GuildId,
        set_name: String,
        choice: String,
        emoji: Option<String>,    
        role_id: RoleId,    
        timestamp: Timestamp,
    ) -> Result<bool> {             
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::UpdateInteractionRoleChoice {
                guild_id,
                set_name,
                choice,
                emoji,
                role_id,
                timestamp,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    } 
    
    pub async fn log_message(
        &self,
        guild_id: GuildId,
        user_id: Option<UserId>,
        channel_id: ChannelId,
        message_id: MessageId,
        timestamp: Timestamp,
        type_: LogType,
        message: Option<Message>,
    ) -> Result<()> {             
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::LogMessage {
                guild_id,
                user_id,
                channel_id,
                message_id,
                timestamp,
                type_,
                message,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    } 

    pub async fn get_message_log(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
        message_id: MessageId,        
    ) -> Result<Vec<MessageLog>> {             
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetLogMessages { 
                guild_id,
                channel_id,
                message_id,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    } 

    pub async fn lookup_user_from_message(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
        message_id: MessageId,        
    ) -> Result<Option<UserId>> {             
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetUserFromLogMessages { 
                guild_id,
                channel_id,
                message_id,
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    } 

    pub async fn db_table_sizes(
        &self,
    ) -> Result<Vec<(String, u64)>> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetTableBytes{
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn get_config_u64(
        &self,
        guild_id: GuildId,
        key: ConfigKey,
    ) -> Result<Option<u64>> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetConfigU64 { 
                guild_id, 
                key, 
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }

    pub async fn get_config_string(
        &self,
        guild_id: GuildId,
        key: ConfigKey,
    ) -> Result<Option<String>> {
        let (s, r) = oneshot::channel();
        self.db_command_sender
            .send_async(DbCommand::GetConfigString { 
                guild_id, 
                key, 
                respond_to: s,
            })
            .await?;
        Ok(r.await??)
    }
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, BotData, Error>;

pub fn configure_tracing() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_line_number(true)
            .with_file(true)
            .finish(),
    )
    .expect("Failed to set default tracing subscriber");
}

pub fn open_database(connection_string: &str, create: bool) -> Result<Connection> {
    let mut open_flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;

    if create {
        open_flags |= OpenFlags::SQLITE_OPEN_CREATE;
    }

    let mut con = Connection::open_with_flags(connection_string, open_flags)?;

    let migrations = Migrations::from_directory(&MIGRATIONS_DIR)?;
    migrations.to_latest(&mut con)?;

    con.pragma_update(None, "journal_mode", "WAL")?;
    con.pragma_update(None, "synchronous", "NORMAL")?;
    con.pragma_update(None, "foreign_keys", "ON")?;

    debug!("Checking DB is writable");
    con.transaction_with_behavior(TransactionBehavior::Exclusive)?;

    Ok(con)
}

pub fn close_database(con: Connection) -> Result<()> {
    con.pragma_update(None, "analysis_limit", "400")?;
    con.pragma_update(None, "optimize", "")?;
    con.pragma_update(None, "foreign_keys", "ON")?;

    if let Err((_con, e)) = con.close() {
        Err(e)?;
    }

    Ok(())
}


#[derive(Debug, Default)]
pub struct PromoteStats {
    pub promoted: usize,
    pub unqualified: usize,
    pub total: usize,
}

impl fmt::Display for PromoteStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Promote {{ promoted: {}, unqualified: {}, total: {} }}",
            self.promoted,
            self.unqualified,
            self.total,
        )
    }
}

pub enum PromoteResult {
    Unconfigured(ConfigKey),
    Ok(PromoteStats)
}

#[allow(unused)]
pub async fn run_promote<'a, 'b, T>(
    data: &'a BotData, 
    ctx: &'a T, 
    guild_id: GuildId,
    force_upgrade_member: Option<Member>,
) -> anyhow::Result<PromoteResult> 
where 
    T: 'a + Clone + CacheHttp + AsRef<Cache> + AsRef<Http>
{    
    const PROGRESS_TITLE: &str = "Promoting";

    macro_rules! get_config_string {
        ($data:expr, $guild_id: expr, $key:expr) => {
            {
                let value = $data
                    .get_config_string($guild_id, $key)
                    .await
                    .with_context(|| format!("Failed to get {} config value", $key))?;

                if value.is_none() {
                    return Ok(PromoteResult::Unconfigured($key));
                }            
            
                value.unwrap()
            }
        };
    }

    macro_rules! get_config_chan {
        ($ctx:expr, $data:expr, $guild_id: expr, $key:expr) => {{
            let string = get_config_string!($data, $guild_id, $key);
            let id = ChannelId::from_str(&string)
                .with_context(|| format!("Failed to parse {} ({}) as a ChannelId", $key, string))?;

            let chan = id.to_channel($ctx)
                .await
                .with_context(|| format!("Failed to resolve {} ({:?}) to a channel", $key, id))?;

            let chan = if let Channel::Guild(c) = chan {
                if c.kind == ChannelType::Text {
                    Some(c)
                } else {
                    None
                }
            } else {
                None
            };
                
            chan.ok_or(anyhow::anyhow!("Channel for {} must be a text channel", $key))?
        }};
    }

    macro_rules! get_config_role {
        ($ctx:expr, $data:expr, $guild_id: expr, $key:expr) => {{
            let string = get_config_string!($data, $guild_id, $key);
            let id = RoleId::from_str(&string)
                .with_context(|| format!("Failed to parse {} ({}) as a RoleId", $key, string))?;
            
            if let Some(role) = id.to_role_cached($ctx) {
                role
            } else {
                // This should warm up the cache only on the first miss
                $guild_id.roles($ctx)
                    .await
                    .context("Failed to lookup guild roles")?;

                id.to_role_cached($ctx)
                    .ok_or(anyhow::anyhow!("Failed to resolve {} ({:?}) to a role", $key, id))?            
            }
        }};
    }

    macro_rules! get_config_u64 {
        ($data:expr, $guild_id: expr, $key:expr) => {{
            let string = get_config_string!($data, $guild_id, $key);
            u64::from_str(&string)
                .with_context(|| format!("Failed to parse {} ({}) as an unsigned int", $key, string))?
        }};
    }

    async fn work<'a, Ctx>(
        ctx: &'a Ctx, 
        (guild_id, data, force_upgrade_member): (GuildId, &'a BotData, Option<Member>), 
        progress_chan: flume::Sender<String>
    ) -> anyhow::Result<PromoteResult> 
    where 
        Ctx: 'a + CacheHttp + AsRef<Http> + AsRef<Cache>,
    {
        /**** Resolve the guild ****/
        let mut guild = guild_id
            .to_guild_cached(ctx)
            .ok_or(anyhow::anyhow!("Guild missing from cache for {:?}", guild_id))?;

        /**** Get all the config we will need  ****/
        let new_role = get_config_role!(ctx, data, guild_id, ConfigKey::GreetRole);
        let junior_role = get_config_role!(ctx, data, guild_id, ConfigKey::PromoteJuniorRole);
        let full_role = get_config_role!(ctx, data, guild_id, ConfigKey::PromoteFullRole);    
        let new_chat_channel = get_config_chan!(ctx, data, guild_id, ConfigKey::PromoteNewChatChannel);
        let junior_chat_channel = get_config_chan!(ctx, data, guild_id, ConfigKey::PromoteJuniorChatChannel);
        let new_chat_min_messages = get_config_u64!(data, guild_id, ConfigKey::PromoteNewChatMinMessages);
        let junior_chat_min_messages = get_config_u64!(data, guild_id, ConfigKey::PromoteJuniorChatMinMessages);
        let junior_min_age = get_config_u64!(data, guild_id, ConfigKey::PromoteJuniorMinAge);        
        let junior_cutoff_age = Utc::now() - Days::new(junior_min_age);

        /**** Do some sanity checks on the config ****/
        anyhow::ensure!(new_role != junior_role, "New and Junior roles cannot be the same! ({:?})", new_role);
        anyhow::ensure!(new_role != full_role, "New and Full roles cannot be the same! ({:?})", new_role);
        anyhow::ensure!(junior_role != full_role, "Junior and Full roles cannot be the same! ({:?})", junior_role);
        anyhow::ensure!(guild.member_count as usize == guild.members.len(), "Member count and number of members in cache differ");

        /**** Fetch the members ****/
        /// TODO: The cache is primed when joining the guild and maintained by events.
        ///       However, this is only good for 1000 or so members. If we reach that
        ///       level this functionality should be converted to be event driven when
        ///       users interact with the server
        let members = guild
            .members
            .iter_mut()
            .filter_map(|(_, m)| if !m.user.bot {
                Some(m)
            } else {
                None
            });

        let mut promote_stats = PromoteStats::default();

        for m in members {
            promote_stats.total += 1;

            let mut promoted = false;
            let is_full = m.roles.contains(&full_role.id);
            let is_new = m.roles.contains(&new_role.id);
            let mut is_junior = m.roles.contains(&junior_role.id);

            let mut skip_checks = if let Some(fum) = force_upgrade_member.as_ref() {
                m.user.id == fum.user.id
            } else {
                false
            };

            if is_full && !is_junior && !is_new {
                continue;
            }

            if is_new {
                let message_count = if skip_checks {
                    new_chat_min_messages
                } else {
                    data.message_count(
                        guild_id.into(),
                        m.user.id.into(),
                        Some(new_chat_channel.id.into()),
                    )
                    .await? as u64
                };

                if message_count >= new_chat_min_messages {
                    if !is_junior && !is_full {
                        progress_chan.send_async(format!("Promoting {} to junior", m)).await;                        
                        m.add_role(ctx, junior_role.id).await
                            .context("Adding junior role")?;
                        is_junior = true;                        
                    }
                    
                    m.remove_role(ctx, new_role.id).await
                        .context("Removing new role")?;

                    promoted |= true;

                    // We only skip 1 round of checks
                    skip_checks = false;
                } else {
                    debug!("Not promoting {} to junior, insufficient messages ({})", m.user.name, message_count);
                }
            }

            if is_junior {
                let old_enough = if let Some(join) = m.joined_at {
                    skip_checks || *join < junior_cutoff_age
                } else {
                    true
                };

                if old_enough {
                    let message_count = if skip_checks {
                        junior_chat_min_messages
                    } else {
                        data.message_count(
                            guild_id.into(),
                            m.user.id.into(),
                            Some(junior_chat_channel.id.into()),
                        )
                        .await? as u64
                    };
                    
                    if message_count >= junior_chat_min_messages {
                        if !is_full {
                            progress_chan.send_async(format!("Promoting {} to full", m)).await;    
                            m.add_role(ctx, full_role.id).await
                                .context("Adding full role")?;
                        }
                        
                        m.remove_role(ctx, junior_role.id).await
                            .context("Removing junior role")?;                        

                        promoted |= true;
                    } else {
                        debug!("Not promoting {} to full, insufficient messages ({})", m.user.name, message_count);
                    }
                } else {
                    debug!("Not promoting {} to full, not been a member long enough", m.user.name);
                }
            }

            if promoted {
                promote_stats.promoted += 1;
            } else {
                promote_stats.unqualified += 1;
            }
        }
        progress_chan.send_async(format!("{}", promote_stats)).await;  

        Ok(PromoteResult::Ok(promote_stats))
    }
    
    with_progress_embed(
        data,
        ctx,
        guild_id,
        LogChannel::General,
        PROGRESS_TITLE,
        work,
        (guild_id, data, force_upgrade_member),
    ).await
}