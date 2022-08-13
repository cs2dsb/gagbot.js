/**
 * Set the role to grant to added members
 *
 * @author Kay <kylrs00@gmail.com>
 * @license ISC - For more information, see the LICENSE.md file packaged with this file.
 * @since r20.2.0
 * @version v1.0.0
 */

const Command = require('../../../command/Command.js');
const { role, channel, num } = require('../../../command/arguments.js');

module.exports = class PromoteRulesCommand extends Command {

    /**
     * GreetRoleCommand constructor
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     */
    constructor() {
        super("promoterules",
            "Set the promotion rules. \nUsage `#new-member-channel-to-scan #junior-member-channel-to-scan min-new-member-messages min-junior-member-messages min-junior-member-age-in-days max-new-member-message-age-in-days`. \nNote about max-new-member-message-age-in-days - this is to stop the bot from requesting every message ever from the new member channel. Recommend setting this to a huge number for the first run then once all the old forgotten accounts are processed set it to something reasonable like 10 days.\nExample: `!promoterules #introduce-yourself #general 1 10 3 30`", "gagbot:promoterrules:set", false, [channel, channel, num, num, num, num]);
    }

    /**
     * Set the role to grant to members with the `am` command.
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
        const gid = message.guild.id;

        const doc = await client.db.guild.findOne({id: gid});
        if (!doc) {
            message.channel.send(`***${client.config.errorMessage}***\n Something went wrong...`);
            console.error(`!promoterules => Couldn't find a guild document with {id: ${gid}}`);
            return true;
        }

        const new_chat_channel = args.get(0);
        const junior_chat_channel = args.get(1);
        const new_chat_min_messages = args.get(2);
        const junior_chat_min_messages = args.get(3);
        const junior_min_age = args.get(4);
        const new_message_max_age = args.get(5);

        doc.data.promoterules = {
            new_chat_channel: new_chat_channel,
            junior_chat_channel: junior_chat_channel,
            new_chat_min_messages: new_chat_min_messages,
            junior_chat_min_messages: junior_chat_min_messages,
            junior_min_age: junior_min_age,
            new_message_max_age: new_message_max_age
        };
        console.log(`!promoterules => doc.data.promoterules: ${ JSON.stringify(doc.data.promoterules)}`);

        doc.markModified('data');
        await doc.save(function(err) {
            if (err) {
                message.channel.send(`***${client.config.errorMessage}***\n Something went wrong...`);
                console.error(`!promoterules => Couldn't save the guild document.`);
            }

            message.channel.send('Promote rules set.');
        });

        return true;
    }
};
