use humansize::{make_format, BINARY};
use poise;
use std::fmt::Write;

use crate::{Context, Error, permissions::{PermissionCheck, Permission}, Embed};

#[poise::command(prefix_command, slash_command, category = "Utils")]
pub async fn help(
    ctx: Context<'_>,

    #[description = "Command to display specific information about"] command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(ctx, command.as_deref(), Default::default()).await?;
    Ok(())
}



#[poise::command(prefix_command, slash_command, category = "Utils")]
/// Get the sizes of each database table
pub async fn get_table_sizes(ctx: Context<'_>) -> Result<(), Error> {
    ctx.require_permission(Permission::ConfigManage).await?;
    
    let mut tables = ctx.data().db_table_sizes().await?;
    tables.sort_by(|a, b| b.1.cmp(&a.1));
    
    let mut max_len = 0;
    for (table, _) in tables.iter() {
        max_len = table.len().max(max_len);
    }
    
    let mut total = 0;
    let mut msg = "```".to_string();
    
    let formatter = make_format(BINARY);
    for (table, size) in tables {
        total += size;
        write!(&mut msg, "{}{}, {}\n", " ".repeat(max_len - table.len()), table, formatter(size))?;
    }
    write!(&mut msg, "{}TOTAL, {}\n", " ".repeat(max_len - "TOTAL".len()), formatter(total))?;

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
pub async fn get_disk_space(ctx: Context<'_>) -> Result<(), Error> {
    ctx.require_permission(Permission::ConfigManage).await?;

    let mut embed = Embed::success()
        .title("Disk space");

    match ctx.data().db_available_space() {
        Ok(bytes) => {
            let formatter = make_format(BINARY);
            embed = embed
                .description(format!("{} available", formatter(bytes)));
        },
        Err(e) => {
            embed = embed
                .set_error(true)
                .description(format!("Error getting available space: {:?}", e));
        }
    }
        
    embed
        .send(&ctx)
        .await?;
    
    Ok(())
}