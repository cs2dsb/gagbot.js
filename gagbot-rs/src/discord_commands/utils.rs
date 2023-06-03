use std::fmt::Write;

use humansize::{make_format, BINARY};

use crate::{
    permissions::{Permission, PermissionCheck},
    Context, Embed, Error,
};

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
pub async fn get_disk_space(ctx: Context<'_>) -> Result<(), Error> {
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
