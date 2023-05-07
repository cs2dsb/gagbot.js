/**
 * Send a greeting to a user in the current channel.
 *
 * @author Kay <kylrs00@gmail.com>
 * @license ISC - For more information, see the LICENSE.md file packaged with this file.
 * @since r20.2.0
 * @version v1.0.0
 */

const Command = require('../../../command/Command.js');
const { user } = require('../../../command/arguments.js');

module.exports = class GreetCommand extends Command {

    /**
     * GreetCommand constructor
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     */
    constructor() {
        super("greet", "Send a greeting to the given user.", "gagbot:greet:send", false, [user]);
    }

    /**
     * Get the greeting from the guild document and send it in the current channel.
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

        // Get the user by ID
        const uid = args.get(0);
        if (!message.guild.members.cache.has(uid)) {
            message.channel.send('No such user.');
            return true;
        }
        const member = message.guild.members.cache.get(uid);
        const user = member.user;

        client.emit('greet', message.guild, user, message.channel);

        // Get the guild doc
        const doc = await client.db.guild.findOne({id: message.guild.id});
        if (!doc) {
            message.channel.send(`***${client.config.errorMessage}***\n Something went wrong...`);
            console.error(`Error while greeting user:\n  Couldn't find a guild document with {id: ${gid}}`);
            return true;
        }

        const guild = message.guild;
        const drid = doc.data.greet.default_role;
        if (drid && guild.roles.cache.has(drid)) {
            const role = guild.roles.cache.get(drid);
    
            member.roles.add(role).catch(function(err) {
                console.error(`Error while adding member role:\n${err}`);
            });
        }

        return true;
    }
};