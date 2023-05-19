use humansize::{make_format, BINARY};
use poise;
use tokio::sync::oneshot;
use std::fmt::Write;

use crate::{Context, Error, permissions::{PermissionCheck, Permission}, DbCommand, Embed};

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
    
    let (s, r) = oneshot::channel();
    ctx.data()
        .db_command_sender
        .send_async(DbCommand::GetTableBytes{
            respond_to: s,
        })
        .await?;

    let formatter = make_format(BINARY);
    
    let mut tables = r.await??;
    tables.sort_by(|a, b| b.1.cmp(&a.1));

    let mut max_len = 0;
    for (table, _) in tables.iter() {
        max_len = table.len().max(max_len);
    }
    
    let mut total = 0;
    let mut msg = "```".to_string();
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
