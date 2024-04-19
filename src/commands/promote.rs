use std::fmt::{self, Debug};

use chrono::{Days, Utc};
use poise::serenity_prelude::{Cache, CacheHttp,  Http, Member};
use tracing::{debug, warn};

use crate::{
    get_config_role, get_config_chan, get_config_u64,
    db::queries::config::{ConfigKey, LogChannel},
    with_progress_embed, BotData, GuildId, ErrorContext, Error, ensure
};

#[derive(Debug, Default)]
pub struct PromoteStats {
    pub promoted: usize,
    pub unqualified: usize,
    pub total: usize,
}

impl fmt::Display for PromoteStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Promote {{ promoted: {}, unqualified: {}, total: {} }}",
            self.promoted, self.unqualified, self.total,
        )
    }
}

pub enum OptionallyConfiguredResult<T> {
    Unconfigured(ConfigKey),
    Ok(T),
}

pub async fn run_promote<'a, 'b, T>(
    data: &'a BotData,
    ctx: &'a T,
    guild_id: GuildId,
    force_upgrade_member: Option<Member>,
) -> Result<OptionallyConfiguredResult<PromoteStats>, Error>
where
    T: 'a + Clone + CacheHttp + AsRef<Cache> + AsRef<Http>,
{
    const PROGRESS_TITLE: &str = "Promoting";

    async fn work<'a, Ctx>(
        ctx: &'a Ctx,
        (guild_id, data, force_upgrade_member): (GuildId, &'a BotData, Option<Member>),
        progress_chan: flume::Sender<String>,
    ) -> Result<OptionallyConfiguredResult<PromoteStats>, Error>
    where
        Ctx: 'a + CacheHttp + AsRef<Http> + AsRef<Cache>,
    {
        // Resolve the guild
        let mut guild = guild_id.to_guild_cached(ctx).ok_or(anyhow::anyhow!(
            "Guild missing from cache for {:?}",
            guild_id
        ))?;

        // Get all the config we will need 
        let new_role = get_config_role!(ctx, data, guild_id, ConfigKey::GreetRole);
        let junior_role = get_config_role!(ctx, data, guild_id, ConfigKey::PromoteJuniorRole);
        let full_role = get_config_role!(ctx, data, guild_id, ConfigKey::PromoteFullRole);
        let new_chat_channel =
            get_config_chan!(ctx, data, guild_id, ConfigKey::PromoteNewChatChannel);
        let junior_chat_channel =
            get_config_chan!(ctx, data, guild_id, ConfigKey::PromoteJuniorChatChannel);
        let new_chat_min_messages =
            get_config_u64!(data, guild_id, ConfigKey::PromoteNewChatMinMessages);
        let junior_chat_min_messages =
            get_config_u64!(data, guild_id, ConfigKey::PromoteJuniorChatMinMessages);
        let junior_min_age = get_config_u64!(data, guild_id, ConfigKey::PromoteJuniorMinAge);
        let junior_cutoff_age = Utc::now() - Days::new(junior_min_age);

        // Do some sanity checks on the config
        ensure!(
            new_role != junior_role,
            "New and Junior roles cannot be the same! ({:?})",
            new_role
        );
        ensure!(
            new_role != full_role,
            "New and Full roles cannot be the same! ({:?})",
            new_role
        );
        ensure!(
            junior_role != full_role,
            "Junior and Full roles cannot be the same! ({:?})",
            junior_role
        );

        // ensure!(
        //     guild.member_count as usize == guild.members.len(),
        //     "Member count and number of members in cache differ"
        // );
        if guild.member_count as usize != guild.members.len() {
            warn!(guild_memeber_count=guild.member_count, guild_members_len=guild.members.len(), "Member count and number of members in cache differ");
        }

        // Fetch the members
        // TODO: The cache is primed when joining the guild and maintained by
        // events.       However, this is only good for 1000 or so
        // members. If we reach that       level this functionality
        // should be converted to be event driven when       users
        // interact with the server
        let members = guild
            .members
            .iter_mut()
            .filter_map(|(_, m)| if !m.user.bot { Some(m) } else { None });

        let mut promote_stats = PromoteStats::default();

        for m in members {
            promote_stats.total += 1;

            let mut promoted = false;
            let is_full = m.roles.contains(&full_role.id);
            let mut is_new = m.roles.contains(&new_role.id);
            let mut is_junior = m.roles.contains(&junior_role.id);

            let mut skip_checks = if let Some(fum) = force_upgrade_member.as_ref() {
                m.user.id == fum.user.id
            } else {
                false
            };

            if is_full && !is_junior && !is_new {
                continue;
            }

            if is_new && is_junior {
                progress_chan
                    .send_async(format!("Removing superfluous {} from {}", new_role.name, m))
                    .await?;

                m.remove_role(ctx, new_role.id)
                    .await
                    .context("Removing new role")?;

                is_new = false;
            }

            if is_junior && is_full {
                progress_chan
                    .send_async(format!("Removing superfluous {} from {}", junior_role.name, m))
                    .await?;

                m.remove_role(ctx, junior_role.id)
                    .await
                    .context("Removing junior role")?;

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
                        progress_chan
                            .send_async(format!("Promoting {} to {}", m, junior_role.name))
                            .await?;
                        m.add_role(ctx, junior_role.id)
                            .await
                            .context("Adding junior role")?;
                        is_junior = true;
                    }

                    m.remove_role(ctx, new_role.id)
                        .await
                        .context("Removing new role")?;

                    promoted |= true;

                    // We only skip 1 round of checks
                    skip_checks = false;
                } else {
                    debug!(
                        "Not promoting {} to junior, insufficient messages ({})",
                        m.user.name, message_count
                    );
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
                            progress_chan
                                .send_async(format!("Promoting {} to {}", m, full_role.name))
                                .await?;
                            m.add_role(ctx, full_role.id)
                                .await
                                .context("Adding full role")?;
                        }

                        m.remove_role(ctx, junior_role.id)
                            .await
                            .context("Removing junior role")?;

                        promoted |= true;
                    } else {
                        debug!(
                            "Not promoting {} to full, insufficient messages ({})",
                            m.user.name, message_count
                        );
                    }
                } else {
                    debug!(
                        "Not promoting {} to full, not been a member long enough",
                        m.user.name
                    );
                }
            }

            if promoted {
                promote_stats.promoted += 1;
            } else {
                promote_stats.unqualified += 1;
            }
        }
        progress_chan.send_async(format!("{}", promote_stats)).await?;

        Ok(OptionallyConfiguredResult::Ok(promote_stats))
    }

    with_progress_embed(
        data,
        ctx,
        guild_id,
        LogChannel::General,
        PROGRESS_TITLE,
        work,
        (guild_id, data, force_upgrade_member),
    )
    .await
}
