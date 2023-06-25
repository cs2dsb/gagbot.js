use std::fmt::Write;

use poise::{self, serenity_prelude::Member};
use tracing::error;

use crate::{
    commands::{add_member::run_add_member, promote::{OptionallyConfiguredResult}},
    permissions::{Permission, PermissionCheck},
    Context, Embed, EmbedFlavour, PoiseError,
};

#[poise::command(prefix_command, slash_command, category = "Member management")]
/// Grant access to a pending member
pub async fn add_member(
    ctx: Context<'_>,
    #[description = "The member to add"]
    member: Member,
) -> Result<(), PoiseError> {
    ctx.defer().await?;
    ctx.require_permission(Permission::MemberAdd).await?;

    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");

    let mut embed = Embed::default().title("Adding member").ephemeral(false);

    let mut msg = String::new();
    // 0 = ok, 1 = warn, 2 = error
    let mut err_lvl = 0;

    let member_name = member.user.name.clone();
    match run_add_member(ctx.data(), &ctx, guild_id.into(), member).await {
        Ok(OptionallyConfiguredResult::Ok(_)) => {
            write!(&mut msg, ":white_check_mark: Added member {}", member_name)?;
        }
        Ok(OptionallyConfiguredResult::Unconfigured(key)) => {
            err_lvl = err_lvl.max(1);
            write!(&mut msg, ":grey_question: Add member not configured: {}", key)?;
        }
        Err(e) => {
            err_lvl = err_lvl.max(2);
            let err = format!(":x: Add member error: {:?}", e);
            error!("Error running add member: {}", err);
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
