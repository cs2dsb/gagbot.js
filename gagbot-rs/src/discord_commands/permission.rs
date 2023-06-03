use std::{borrow::Cow, fmt::Write};

use poise::{
    self,
    serenity_prelude::{ButtonStyle, Member, RoleId},
};

use crate::{
    permissions::{Permission, PermissionCheck},
    Context, Embed, EmbedFlavour, Error,
};

#[poise::command(prefix_command, slash_command, guild_only, category = "Permission")]
/// Print the effective permissions for a given user
pub async fn get_permissions(
    ctx: Context<'_>,

    #[description = "The member to get permissions for. Defaults to your member if not provided"]
    member: Option<Member>,
) -> Result<(), Error> {
    ctx.require_permission(Permission::PermissionManage).await?;

    let guild = ctx.guild().expect("missing guild in 'guild_only' command");

    let (msg, err) = if let Some(member) = member
        .map(|v| Cow::Owned(v))
        .or(ctx.author_member().await.map(|v| v.clone()))
    {
        if guild.owner_id == member.user.id {
            ("Server owner has all permissions".to_string(), false)
        } else {
            match ctx.data().get_member_permissions(&guild, &member).await {
                Ok(permissions) => {
                    let mut msg = format!("Effective permissions:");
                    for p in permissions.iter() {
                        write!(&mut msg, "\n{}", p)?;
                    }
                    (msg, false)
                }
                Err(e) => (
                    format!("Error fetching permissions for {}: {:?}", member, e),
                    true,
                ),
            }
        }
    } else {
        (
            "Member not provided and author missing, was this a DM outside the guild?".to_string(),
            true,
        )
    };

    Embed::default()
        .description(msg)
        .set_error(err)
        .send(&ctx)
        .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Permission")]
/// Grant a permission for a given role
pub async fn grant_permission(
    ctx: Context<'_>,

    #[description = "The role to grant the permission"] role: RoleId,
    #[description = "The permission to grant"] permission: Permission,
) -> Result<(), Error> {
    ctx.require_permission(Permission::PermissionManage).await?;

    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");
    let timestamp = ctx.created_at();

    let (msg, err) = match ctx
        .data()
        .grant_permission(guild_id.into(), role.into(), permission, timestamp)
        .await
    {
        Ok(true) => ("Granted".to_string(), false),
        Ok(false) => ("Was already granted :person_shrugging:".to_string(), false),
        Err(e) => (format!("Error granting permission: {:?}", e), true),
    };

    Embed::default()
        .description(msg)
        .set_error(err)
        .send(&ctx)
        .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Permission")]
/// Revoke a permission for a given role
pub async fn revoke_permission(
    ctx: Context<'_>,

    #[description = "The role to revoke the permission from"] role: RoleId,
    #[description = "The permission to revoke"] permission: Permission,
) -> Result<(), Error> {
    ctx.require_permission(Permission::PermissionManage).await?;

    let guild_id = ctx
        .guild_id()
        .expect("missing guild in 'guild_only' command");
    let timestamp = ctx.created_at();

    let (msg, err) = match ctx
        .data()
        .revoke_permission(guild_id.into(), role.into(), permission, timestamp)
        .await
    {
        Ok(true) => ("Revoked".to_string(), false),
        Ok(false) => ("Was already revoked :person_shrugging:".to_string(), false),
        Err(e) => (format!("Error revoking permission: {:?}", e), true),
    };

    Embed::default()
        .description(msg)
        .set_error(err)
        .send(&ctx)
        .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, category = "Permission")]
/// Purge all permissions
pub async fn purge_permission(ctx: Context<'_>) -> Result<(), Error> {
    ctx.require_permission(Permission::PermissionManage).await?;

    let guild_id = ctx
        .guild_id()
        .expect("missing guild_id in 'guild_only' command");

    let timestamp = ctx.created_at();

    let reply = ctx.send(|m| m
        .embed(|b| Embed::default()
            .description("Are you sure you want for purge all permissions. If you aren't the server owner you will be locked out of the bot")
            .create_embed(b)
        )
        .ephemeral(true)
        .components(|c| c
            .create_action_row(|r| r
                .create_button(|b| b
                    .custom_id("purge.ok")
                    .label("Purge")
                    .style(ButtonStyle::Danger)
                )
                .create_button(|b| b
                    .custom_id("purge.cancel")
                    .label("Cancel")
                    .style(ButtonStyle::Secondary)
                )
            )
        )
    ).await?;

    let interaction = reply
        .message()
        .await?
        .await_component_interaction(ctx)
        .author_id(ctx.author().id)
        .await;

    // Remove the buttons once one is clicked
    reply
        .edit(ctx, |b| {
            b.components(|b| b).embed(|b| {
                Embed::default()
                    .description("Processing...")
                    .create_embed(b)
            })
        })
        .await?;

    let do_purge = match &interaction {
        Some(b) => b.data.custom_id == "purge.ok",
        None => {
            reply
                .edit(ctx, |b| {
                    b.components(|b| b).embed(|b| {
                        Embed::error()
                            .description("Interaction timed out")
                            .create_embed(b)
                    })
                })
                .await?;
            return Ok(());
        }
    };

    let (msg, err) = if do_purge {
        match ctx
            .data()
            .purge_permission(guild_id.into(), timestamp)
            .await
        {
            Ok(true) => ("Purged".to_string(), Some(false)),
            Ok(false) => (
                "Nothing to purge :person_shrugging:".to_string(),
                Some(false),
            ),
            Err(e) => (format!("Error purging permission: {:?}", e), Some(true)),
        }
    } else {
        ("Cancelled".to_string(), None)
    };

    reply
        .edit(ctx, |b| {
            b.components(|b| b).embed(|b| {
                Embed::default()
                    .description(msg)
                    .flavour(match err {
                        Some(false) => EmbedFlavour::Success,
                        Some(true) => EmbedFlavour::Error,
                        None => EmbedFlavour::Normal,
                    })
                    .create_embed(b)
            })
        })
        .await?;

    Ok(())
}
