use std::fmt::{Write, Display};

use humansize::{make_format, BINARY};

use crate::{
    permissions::{Permission, PermissionCheck},
    Context, Embed, PoiseError, db::queries::config::ConfigKey, get_config_u64_option, get_config_role_option, get_config_chan_option, get_config_string_option, Error,
};

#[poise::command(prefix_command, slash_command, category = "Utils")]
pub async fn help(
    ctx: Context<'_>,

    #[description = "Command to display specific information about"] command: Option<String>,
) -> Result<(), PoiseError> {
    poise::builtins::help(ctx, command.as_deref(), Default::default()).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "Utils")]
/// Get the sizes of each database table
pub async fn get_table_sizes(ctx: Context<'_>) -> Result<(), PoiseError> {
    ctx.require_permission(Permission::ConfigManage).await?;

    let mut tables = ctx.data().db_table_sizes().await?;
    tables.sort_by(|a, b| b.1.cmp(&a.1));

    let formatter = make_format(BINARY);
    let mut total = 0;

    let tables = tables
        .into_iter()
        .map(|(name, size, count)| {
            total += size;
            (name, formatter(size), count)
        })
        .collect::<Vec<(_, String, _)>>();

    let mut max_table_len = 0;
    let mut max_size_len = 0;
    for (table, size, _) in tables.iter() {
        max_table_len = (table.len() + 1).max(max_table_len);
        max_size_len = (size.len() + 1).max(max_size_len);
    }

    let mut msg = "```".to_string();

    macro_rules! pad {
        ($msg:expr, $len:expr) => {
            write!(&mut msg, "{}{}", " ".repeat($len - $msg.len()), $msg)?;
        };
    }

    pad!("Table,", max_table_len + 1);
    pad!("Size,", max_size_len + 1);
    write!(&mut msg, " Row count\n")?;

    for (table, size, count) in tables {
        pad!(&table, max_table_len);
        write!(&mut msg, ",")?;
        pad!(&size, max_size_len);
        write!(&mut msg, ", {}\n", count)?;
    }
    pad!("Total:", max_table_len + 1);
    write!(&mut msg, " {}\n", formatter(total))?;

    msg.push_str("```");

    Embed::success()
        .title("Table sizes")
        .description(msg)
        .send(&ctx)
        .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "Utils")]
/// Get the free space on the disk the database file is located on
pub async fn get_disk_space(ctx: Context<'_>) -> Result<(), PoiseError> {
    ctx.require_permission(Permission::ConfigManage).await?;

    let mut embed = Embed::success().title("Disk space");

    match ctx.data().db_available_space() {
        Ok(bytes) => {
            let formatter = make_format(BINARY);
            embed = embed.description(format!("{} available", formatter(bytes)));
        }
        Err(e) => {
            embed = embed
                .set_error(true)
                .description(format!("Error getting available space: {:?}", e));
        }
    }

    embed.send(&ctx).await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "Utils")]
/// Check all configurable features
pub async fn check_config(ctx: Context<'_>) -> Result<(), PoiseError> {
    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command")
        .into();
    
    ctx.require_permission(Permission::ConfigManage).await?;

    const EMOJI_RED_X: &str = ":x:";
    const EMOJI_GREEN_TICK: &str = ":white_check_mark:";

    let mut msg = "".to_string();    

    fn check_cfg<T: Display, W: Write>(cfg: Option<T>, key: ConfigKey, mut msg: W) -> Result<(), Error> {
        let name = key.name();
        let description = key.description();
        let (emoji, not, value) = if let Some(cfg) = cfg {
            (EMOJI_GREEN_TICK, "", Some(cfg.to_string()))
        } else {
            (EMOJI_RED_X, "not ", None)
        };

        write!(msg, "## {emoji} {name} {not}configured\n> {description}\n")?;
        if let Some(value) = value {
            write!(msg, "*Value:* {value}\n")?;
        }
        Ok(())
    }

    let data = ctx.data();
    
    write!(&mut msg, "# Greet config\n")?;
    check_cfg(
        get_config_string_option!(data, guild_id, ConfigKey::GreetMessage),
        ConfigKey::GreetMessage,
        &mut msg)?;
        
    check_cfg(
        get_config_chan_option!(ctx, data, guild_id, ConfigKey::GreetChannel),
        ConfigKey::GreetChannel,
        &mut msg)?;
        
    check_cfg(
        get_config_string_option!(data, guild_id, ConfigKey::GreetWelcomeMessage),
        ConfigKey::GreetWelcomeMessage,
        &mut msg)?;
        
    check_cfg(
        get_config_chan_option!(ctx, data, guild_id, ConfigKey::GreetWelcomeChannel),
        ConfigKey::GreetWelcomeChannel,
        &mut msg)?;
        
    check_cfg(
        get_config_role_option!(ctx, data, guild_id, ConfigKey::GreetRole),
        ConfigKey::GreetRole,
        &mut msg)?;
        
    check_cfg(
        get_config_role_option!(ctx, data, guild_id, ConfigKey::GreetDefaultRole),
        ConfigKey::GreetDefaultRole,
        &mut msg)?;


    write!(&mut msg, "# Logging config\n")?;        
    check_cfg(
        get_config_chan_option!(ctx, data, guild_id, ConfigKey::LoggingGeneral),
        ConfigKey::LoggingGeneral,
        &mut msg)?;
        
    check_cfg(
        get_config_chan_option!(ctx, data, guild_id, ConfigKey::LoggingEditsAndDeletes),
        ConfigKey::LoggingEditsAndDeletes,
        &mut msg)?;
        
    check_cfg(
        get_config_chan_option!(ctx, data, guild_id, ConfigKey::LoggingJoiningAndLeaving),
        ConfigKey::LoggingJoiningAndLeaving,
        &mut msg)?;
        
    check_cfg(
        get_config_chan_option!(ctx, data, guild_id, ConfigKey::LoggingErrors),
        ConfigKey::LoggingErrors,
        &mut msg)?;
        
    check_cfg(
        get_config_chan_option!(ctx, data, guild_id, ConfigKey::LoggingVoiceActivity),
        ConfigKey::LoggingVoiceActivity,
        &mut msg)?;

    write!(&mut msg, "# Promote config\n")?; 
        
    check_cfg(
        get_config_role_option!(ctx, data, guild_id, ConfigKey::PromoteJuniorRole),
        ConfigKey::PromoteJuniorRole,
        &mut msg)?;
        
    check_cfg(
        get_config_role_option!(ctx, data, guild_id, ConfigKey::PromoteFullRole),
        ConfigKey::PromoteFullRole,
        &mut msg)?;
        
    check_cfg(
        get_config_chan_option!(ctx, data, guild_id, ConfigKey::PromoteNewChatChannel),
        ConfigKey::PromoteNewChatChannel,
        &mut msg)?;
        
    check_cfg(
        get_config_chan_option!(ctx, data, guild_id, ConfigKey::PromoteJuniorChatChannel),
        ConfigKey::PromoteJuniorChatChannel,
        &mut msg)?;
        
    check_cfg(
        get_config_u64_option!(data, guild_id, ConfigKey::PromoteNewChatMinMessages),
        ConfigKey::PromoteNewChatMinMessages,
        &mut msg)?;
        
    check_cfg(
        get_config_u64_option!(data, guild_id, ConfigKey::PromoteJuniorChatMinMessages),
        ConfigKey::PromoteJuniorChatMinMessages,
        &mut msg)?;
        
    check_cfg(
        get_config_u64_option!(data, guild_id, ConfigKey::PromoteJuniorMinAge),
        ConfigKey::PromoteJuniorMinAge,
        &mut msg)?;
        

    Embed::default()
        .description(msg)
        .send(&ctx)
        .await?;

    Ok(())
}
