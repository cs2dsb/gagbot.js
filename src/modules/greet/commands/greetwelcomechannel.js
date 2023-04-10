/**
 * Set the welcome channel
 *
 * @author Kay <kylrs00@gmail.com>
 * @license ISC - For more information, see the LICENSE.md file packaged with this file.
 * @since r20.2.0
 * @version v1.0.0
 */

const Command = require('../../../command/Command.js');
const { channel } = require('../../../command/arguments.js');

module.exports = class GreetWelcomeChannelCommand extends Command {

    /**
     * GreetRoleCommand constructor
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     */
    constructor() {
        super("greetwelcomechannel", "Set the welcome channel.", "gagbot:greet:set", false, [channel]);
    }

    /**
     * Set the channel to welcome members in with the `am` command.
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
            console.error(`Error while setting the welcome channel:\n  Couldn't find a guild document with {id: ${gid}}`);
            return true;
        }

        if (!doc.data.greet) doc.data.greet = {};
        doc.data.greet.welcomechannel = args.get(0);

        doc.markModified('data');
        await doc.save(function(err) {
            if (err) {
                message.channel.send(`***${client.config.errorMessage}***\n Something went wrong...`);
                console.error(`Error while setting the welcome channel:\n  Couldn't save the guild document.`);
            }

            message.channel.send('Welcome channel set.');
        });

        return true;
    }
};
