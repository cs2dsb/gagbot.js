/**
 * Define `logging` module events
 *
 * @author Kay <kylrs00@gmail.com>
 * @license ISC - For more information, see the LICENSE.md file packaged with this file.
 * @since r20.2.0
 * @version v1.0.3
 */

const Logger = require('./Logger.js');
const { AuditLogEvent } = require('discord.js');

module.exports = {
    /**
     * Initialise the logger
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     *
     * @param {Client} client
     */
    async on_ready(client) {
        client.logger = new Logger(client);
    },


      ////////////////////
     // MESSAGE events //
    ////////////////////

    /**
     * Log when a user edits a message
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     *
     * @param {Client} client
     * @param {Message} before
     * @param {Message} after
     */
    async on_messageUpdate(client, before, after) {
        // console.log(`on_messageUpdate: ${before} => ${after}`);
        const user = before.author;
        if (user.bot) return;
        if (before.content !== after.content) {
            await client.logger.log(
                before.guild,
                'message',
                `\`${user.username}#${user.discriminator}\` edited their message.`,
                `**Before**\n\`\`\`\n${before.content}\n\`\`\`\n` +
                `**After**\n\`\`\`\n${after.content}\n\`\`\``,
                0x30649c,
                [
                    { name: 'Channel', value: before.channel.toString()},
                    { name: 'Timestamp', value: `\`${new Date().toLocaleString()}\``},
                ],
            );
        }
    },

    /**
     * Log when a user deletes a message
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     *
     * @param {Client} client
     * @param {Message} message
     */
    async on_messageDelete(client, message) {
        // console.log(`on_messageDelete: ${message}`);
        const user = message.author;
        await client.logger.log(
            message.guild,
            'message',
            `Message from \`${user.username}#${user.discriminator}\` was deleted.`,
            `\`\`\`\n${message.content}\n\`\`\`\n`,
            0x9c3730,
            [
                { name: 'Channel', value: message.channel.toString()},
                { name: 'Timestamp', value: `\`${new Date().toLocaleString()}\``},
            ],
        );
    },


      //////////////////
     // VOICE events //
    //////////////////

    /**
     * Log when a joins, leaves, or moves to a different voice channel
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     *
     * @param {Client} client
     * @param {VoiceState} before
     * @param {VoiceState} after
     */
    async on_voiceStateUpdate(client, before, after) {
        // console.log(`on_voiceStateUpdate: ${before} => ${after}`);
        if (before.member.bot || after.member.bot) return;

        let msg = '';
        let colour = undefined;
        const fields = [];

        if (before.channel) {
            if (!after.channel) {
                msg = 'Left voice chat.';
                fields.push({ name: 'Channel', value: before.channel.toString()});
                colour = 0xff4d64;
            } else if (before.channelID !== after.channelID) {
                msg = 'Changed channels.';
                fields.push({ name: 'From', value: before.channel.toString()});
                fields.push({ name: 'To', value: after.channel.toString()});
                colour = 0x4d8bff;
            }
        } else if (after) {
            msg = 'Joined voice chat.';
            fields.push({ name: 'Channel', value: after.channel.toString()});
            colour = 0x4dff9d;
        }

        if (msg.length) {
            fields.push({ name: 'User', value: before.member.user.toString()});
            fields.push({ name: 'Timestamp', value: `\`${new Date().toLocaleString()}\``});

            await client.logger.log(
                before.guild,
                'voice',
                `Voice State update for \`${before.member.user.username}#${before.member.user.discriminator}\``,
                msg, colour, fields,
            );
        }
    },


      ///////////////////
     // MEMBER events //
    ///////////////////

    /**
     * Log when a user joins a guild
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     *
     * @param {Client} client
     * @param {GuildMember} member
     */
    async on_guildMemberAdd(client, member) {
        // console.log(`on_guildMemberAdd: ${member}`);
        const user = member.user;
        await client.logger.log(
            member.guild,
            'member',
            `\`${user.username}#${user.discriminator}\` joined the server.`,
            '',
            0x009900,
            [{ name: 'Timestamp', value: `\`${new Date().toLocaleString()}\`` }],
        );
    },

    /**
     * Log when a user leaves a guild
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     *
     * @param {Client} client
     * @param {GuildMember} member
     */
    async on_guildMemberRemove(client, member) {
        // console.log(`on_guildMemberRemove: ${member}`);
        const user = member.user;

        member.guild
            .fetchAuditLogs({type: AuditLogEvent.MemberKick})
            .then(async (logs) => {
                const now = new Date();
                // Find a recent kick audit, targeting the user who just left
                const kick = logs.entries.find((entry) => {
                    return entry.target === user
                        && now - entry.createdAt < 1e4;
                });

                if (kick) {
                    await client.logger.log(
                        member.guild,
                        'member',
                        `\`${user.username}#${user.discriminator}\` was kicked from the server.`,
                        '',
                        0x990044,
                        [
                            { name: 'By', value: kick.executor.toString() },
                            { name: 'Timestamp', value: `\`${new Date().toLocaleString()}\`` },
                        ],
                    );
                } else {
                    // If no kick audit was found, assume the user left of their own accord
                    await client.logger.log(
                        member.guild,
                        'member',
                        `\`${user.username}#${user.discriminator}\` left the server.`,
                        '',
                        0x990000,
                        [{ name: 'Timestamp', value: `\`${new Date().toLocaleString()}\`` }],
                    );
                }
            })
            .catch((err) => {
                console.error(err);
            });


    },
};
