use std::fmt::Write;

use poise::{self, serenity_prelude::Member};
use tracing::error;

use crate::{
    commands::promote::{run_promote, OptionallyConfiguredResult},
    db::queries::permissions::{Permission, PermissionCheck},
    Context, Embed, EmbedFlavour, PoiseError,
};

#[poise::command(prefix_command, slash_command, category = "Member management")]
/// Manually kicks off promotion process
pub async fn promote(
    ctx: Context<'_>,
    #[description = "Override checks and upgrade this member to the next level"]
    force_upgrade_member: Option<Member>,
) -> Result<(), PoiseError> {
    ctx.defer_ephemeral().await?;
    ctx.require_permission(Permission::MemberPromote).await?;

    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");

    let mut embed = Embed::default().title("Promoting").ephemeral(true);

    let mut msg = String::new();
    // 0 = ok, 1 = warn, 2 = error
    let mut err_lvl = 0;

    match run_promote(ctx.data(), &ctx, guild_id.into(), force_upgrade_member).await {
        Ok(OptionallyConfiguredResult::Ok(promotions)) => {
            write!(&mut msg, ":white_check_mark: {}", promotions)?;
        }
        Ok(OptionallyConfiguredResult::Unconfigured(key)) => {
            err_lvl = err_lvl.max(1);
            write!(&mut msg, ":grey_question: Promote not configured: {}", key)?;
        }
        Err(e) => {
            err_lvl = err_lvl.max(2);
            let err = format!(":x: Promote error: {:?}", e);
            error!("Error running promotions: {}", err);
            msg.push_str(&err);
        }
    }
    msg.push('\n');

    embed.flavour = Some(match err_lvl {
        0 => EmbedFlavour::Success,
        1 => EmbedFlavour::Normal,
        _ => EmbedFlavour::Error,
    });

    embed = embed.description(msg);

    embed.send(&ctx).await?;

    Ok(())
}
