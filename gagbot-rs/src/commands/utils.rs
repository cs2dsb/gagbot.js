use poise;

use crate::{Context, Error};

#[poise::command(prefix_command, slash_command, category = "Utils")]
pub async fn help(
    ctx: Context<'_>,

    #[description = "Command to display specific information about"] command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(ctx, command.as_deref(), Default::default()).await?;
    Ok(())
}
