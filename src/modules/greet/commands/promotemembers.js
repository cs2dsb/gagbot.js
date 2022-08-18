/**
 * Grant a "member" role to a user. Designed to facilitate waiting-room channels.
 *
 * @author Kay <kylrs00@gmail.com>
 * @license ISC - For more information, see the LICENSE.md file packaged with this file.
 * @since r20.2.0
 * @version v1.0.1
 */

const Command = require('../../../command/Command.js');
const GagEmbed = require('../../../responses/GagEmbed.js');
const { EmbedBuilder, ChannelType } = require('discord.js');

module.exports = class PromoteMemberCommand extends Command {

    /**
     * AddMemberCommand constructor
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     */
    constructor() {
        super("promote", "Promote eligible members: New to junior members when they have > 2 roles set. Junior to full when they've been active on the server for > 3 days", "gagbot:greet:promotemembers", false, []);
    }

    /**
     * Grant the "memberRole" to the tagged user
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     *
     * @param {Client} client
     * @param {Message} message
     * @param {ArgumentList} args
     * @returns {boolean}
     */
    async execute(client, message, args) {
        const guild = message.guild;
        const initiating_user = message.author;

        // Get the guild doc
        const doc = await client.db.guild.findOne({id: guild.id});
        if (!doc) {
            message.channel.send(`***${client.config.errorMessage}***\n Something went wrong...`);
            console.error(`!promote => Couldn't find a guild document with {id: ${gid}}`);
            return true;
        }

        // Check required config
        const new_rid = doc.data?.greet?.role;
        const junior_rid = doc.data?.promoteroles?.junior_role;
        const full_rid = doc.data?.promoteroles?.full_role;

        const junior_chat_channel_id = doc.data?.promoterules?.junior_chat_channel;
        const new_chat_channel_id = doc.data?.promoterules?.new_chat_channel;
        const new_chat_min_messages = doc.data?.promoterules?.new_chat_min_messages;
        const junior_chat_min_messages = doc.data?.promoterules?.junior_chat_min_messages;
        const junior_min_age = doc.data?.promoterules?.junior_min_age;


        let bail = false;
        const check_arg = (value, name, kind) => {
            if (value === null || value === undefined) {
                message.channel.send(`${kind} ${name} not configured`);
                bail = true;
            }
        };

        check_arg(new_rid, "new member", "role");
        check_arg(junior_rid, "junior member", "role");
        check_arg(full_rid, "full member", "role");
        check_arg(junior_chat_channel_id, "junior chat channel", "channel");
        check_arg(new_chat_channel_id, "new chat channel", "channel");
        check_arg(new_chat_min_messages, "new min messages", "number");
        check_arg(junior_chat_min_messages, "junior min messages", "number");
        check_arg(junior_min_age, "junior min age", "channel");

        if (new_rid == junior_rid) { message.channel.send('New and Junior roles are THE SAME'); bail = true;}
        if (new_rid == full_rid) { message.channel.send('New and Member roles are THE SAME'); bail = true;}
        if (junior_rid == full_rid) { message.channel.send('Junior and Member roles are THE SAME'); bail = true;}

        if (bail) {
            return true;
        }
        console.log(`!promote => new_rid: ${new_rid}, junior_rid: ${junior_rid}, full_rid: ${full_rid}`);

        const embed = new GagEmbed('Calculating promotions...', '', {});

        message.channel
            .send({ embeds: [embed]})
            .then((embed) => {
                let cleanup_done = false;
                const cleanup = () => {
                    if (cleanup_done) { return; }
                    cleanup_done = true;
                    embed.delete();
                };

                // Resolve the roles & channels
                const resolve_role = (id, name) => {
                    if (!guild.roles.cache.has(id)) {
                        message.channel.send(`***${client.config.errorMessage}***\n Something went wrong... (Failed to lookup ${name} role)`);
                        console.error(`!promote => Error resolving member role:\n No such role (${name}).`);
                        return null;
                    }
                    return guild.roles.cache.get(id);
                };
                const resolve_channel = (id, name) => {
                    if (!guild.channels.cache.has(id)) {
                        message.channel.send(`***${client.config.errorMessage}***\n Something went wrong... (Failed to lookup ${name} channel)`);
                        console.error(`!promote => Error resolving guild channel:\n No such channel (${name}).`);
                        return null;
                    }
                    const channel = guild.channels.cache.get(id);
                    if (channel.type !== ChannelType.GuildText) {
                        message.channel.send(`***${client.config.errorMessage}***\n Something went wrong... (${name} channel is not a text channel)`);
                        console.error(`!promote => Error resolving guild channel:\n Not a text channel (${name}).`);
                        return null;
                    }
                    return channel;
                };

                const new_role = resolve_role(new_rid, "new_role");
                const junior_role = resolve_role(junior_rid, "junior_role");
                const full_role = resolve_role(full_rid, "full_role");
                const junior_chat_channel = resolve_channel(junior_chat_channel_id, "junior_chat_channel");
                const new_chat_channel = resolve_channel(new_chat_channel_id, "new_chat_channel");

                if (!new_role || !junior_role || !full_role || !junior_chat_channel) {
                    console.error(`!promote => Failed to resolve roles/channels. new_role: ${new_role}, junior_role: ${junior_role}, full_role: ${full_role}, junior_chat_channel: ${junior_chat_channel}`);
                    cleanup();
                    return;
                }

                guild.members
                    .fetch()
                    .then(async (members) => {
                        const dangling_new = [];
                        const new_to_junior = [];
                        const dangling_junior = [];
                        const junior_to_full = [];

                        const junior_cutoff_date = new Date();
                        junior_cutoff_date.setDate(junior_cutoff_date.getDate() - junior_min_age);

                        [...members.values()]
                            .filter((member) => !member.deleted)
                            .forEach((member) => {
                                const id = member.id;
                                const name = member.displayName;
                                const join_date = member.joinedAt;
                                const is_new = member.roles.cache.has(new_rid);
                                const is_junior = member.roles.cache.has(junior_rid);
                                const is_full = member.roles.cache.has(full_rid);

                                let skip = false;

                                if (is_full && is_junior) {
                                    dangling_junior.push(member);
                                    skip = true;
                                }

                                if ((is_full || is_junior) && is_new) {
                                    dangling_new.push(member);
                                    skip = true;
                                }

                                // Don't do anything else until they are cleaned up
                                if (skip) { return; }

                                if (is_new && member.roles.cache.size > 2) {
                                    new_to_junior.push(member);
                                }

                                if (is_junior && join_date < junior_cutoff_date) {
                                    junior_to_full.push(member);
                                }
                            });



                        const count_messages = async (channel, role, members, min) => {
                            const new_embed = new GagEmbed(`Checking ${channel.name} message counts to assess @${role.name} participation (>= ${min})`, '', {});
                            await embed.edit({ embeds: [ new_embed ] });

                            for (let i = members.length - 1; i >= 0; i--) {
                                const member = members[i];

                                const query = {
                                    guild: message.guild.id,
                                    user: member.id
                                };
                                query[`channel_message_counts.${channel.id}`] = { '$gte': min };

                                const result = await client.db.activityLog.findOne(query, {"_id" : 1});
                                if (result === null || result === undefined) {
                                    console.log(`!promote => ${member.displayName} hasn't spoken. Skipping promotion`);
                                    members.splice(i, 1);
                                }
                            }
                        };


                        if (new_to_junior.length > 0) {
                            // Check if they have introduced themselves
                            await count_messages(new_chat_channel, new_role, new_to_junior, new_chat_min_messages);
                        }

                        if (junior_to_full.length > 0) {
                            // Check if they have participated
                            await count_messages(junior_chat_channel, junior_role, junior_to_full, junior_chat_min_messages);
                        }

                        cleanup();

                        let nothing_to_do = true;

                        const offer_action = (title, list, action) => {
                            if (list.length == 0) { return; }
                            nothing_to_do = false;

                            const desc = (footer) => {
                                const d = `This action will effect ${list.length} members:\n`
                                + list + '\n'
                                + ((footer != null && footer != undefined && footer.length > 0) ? footer : `***React ✅ to proceed, or 🚫 to cancel.***`);

                                return d;
                            };

                            const confirm_embed = new GagEmbed(title, desc());

                            message.channel.send({ embeds: [confirm_embed]})

                            .then((message) => {
                                message.react('🚫')
                                    .then(() => message.react('✅'))
                                    .then(() => {
                                        const cleanup = () => {
                                            message.reactions.removeAll();
                                        };

                                        const filter = (reaction, user) => {
                                            // console.log(`reaction_filter: reaction: ${reaction.emoji.name}, user: ${user}`);
                                            return ['🚫', '✅'].includes(reaction.emoji.name) && user.id === initiating_user.id;
                                        };

                                        message.awaitReactions({ filter, max: 1, time: 5*6*1000, errors: ['time'] })
                                            .then((collected) => {
                                                const reaction = collected.first();

                                                cleanup();
                                                if (reaction.emoji.name === '✅') {
                                                    confirm_embed.setDescription(desc("Changes confirmed and applied"));
                                                    message.edit({ embeds: [ confirm_embed ]});
                                                    //message.channel.send({ embeds: [new EmbedBuilder().setTitle(title + ' CONFIRMED.').setColor(0x92fc68)]});
                                                    list.forEach(async (member) => action(member));
                                                } else {
                                                    confirm_embed.setDescription(desc("Changes cancelled"));
                                                    message.edit({ embeds: [ confirm_embed ]});
                                                    // message.channel.send({ embeds: [new EmbedBuilder().setTitle(title + ' CANCELLED.').setColor(0xfc687e)]});
                                                }
                                            })
                                            .catch(() => {
                                                cleanup();
                                                confirm_embed.setDescription(desc("Changes cancelled (timeout)"));
                                                message.edit({ embeds: [ confirm_embed ]});
                                                // message.channel.send({ embeds: [new EmbedBuilder().setTitle(title + ' CANCELLED (timeout).').setColor(0xfc687e)]});
                                            });
                                    });
                            })
                            .catch((err) => {
                                console.error(err);
                            });
                        };

                        const remove_role = (role) => {
                            return (member) => {
                                console.log(`!promote => Removing @${role.name} from ${member.displayName}...`);
                                member.roles.remove(role).then(() => {
                                    console.log(`!promote =>   ...done removing @${role.name} from ${member.displayName}`);
                                }).catch((err) => {
                                    console.log(`!promote =>   ...error removing @${role.name} from ${member.displayName}`);
                                    console.error(err);
                                });
                            };
                        };

                        const swap_role = (remove_role, add_role) => {
                            return (member) => {
                                console.log(`!promote => Swapping @${remove_role.name} to @${add_role.name} for ${member.displayName}...`);
                                member.roles.add(add_role)
                                    .then(() => {
                                        member.roles.remove(remove_role)
                                            .then(() => console.log(`!promote =>   ...done swapping @${remove_role.name} to @${add_role.name} for ${member.displayName}`))
                                            .catch((err) => {
                                                console.log(`!promote =>   ...error removing @${remove_role.name} from ${member.displayName}`);
                                                console.error(err);
                                            });
                                    })
                                    .catch((err) => {
                                        console.log(`!promote =>   ...error adding @${add_role.name} to ${member.displayName}`);
                                        console.error(err);
                                    });
                            };
                        };

                        offer_action(`Cleaning up unneeded @${new_role.name} roles`, dangling_new, remove_role(new_role));
                        offer_action(`Cleaning up unneeded @${junior_role.name} roles`, dangling_junior, remove_role(junior_role));
                        offer_action(`Swapping @${new_role.name} role to @${junior_role.name}`, new_to_junior, swap_role(new_role, junior_role));
                        offer_action(`Swapping @${junior_role.name} role to @${full_role.name}`, junior_to_full, swap_role(junior_role, full_role));

                        if (nothing_to_do) {
                            message.channel.send({ embeds: [new EmbedBuilder().setTitle("All up-to-date, no changes required :)").setColor(0x92fc68)]});
                            console.log("!promote => nothing to do");
                        }
                    })
                    .catch((err) => {
                        console.error(err);
                        cleanup();
                    });
            });

        return true;
    }
};
