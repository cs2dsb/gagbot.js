/**
 * Set the role to grant to added members
 *
 * @author Kay <kylrs00@gmail.com>
 * @license ISC - For more information, see the LICENSE.md file packaged with this file.
 * @since r20.2.0
 * @version v1.0.0
 */

const Command = require('../../../command/Command.js');
const { role } = require('../../../command/arguments.js');

module.exports = class PromoteRolesCommand extends Command {

    /**
     * GreetRoleCommand constructor
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     */
    constructor() {
        super("promoteroles", "Set the promotion member roles. Example: `!promoteroles @Junior Member @Member`", "gagbot:promoteroles:set", false, [role, role]);
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
            console.error(`!promoteroles => Couldn't find a guild document with {id: ${gid}}`);
            return true;
        }

        const junior_role = args.get(0);
        const full_role = args.get(1);

        doc.data.promoteroles = {
            junior_role: junior_role,
            full_role: full_role
        };
        console.log(`!promoteroles => doc.data.promoteroles: ${ JSON.stringify(doc.data.promoteroles)}`);

        doc.markModified('data');
        await doc.save(function(err) {
            if (err) {
                message.channel.send(`***${client.config.errorMessage}***\n Something went wrong...`);
                console.error(`!promoteroles => Couldn't save the guild document.`);
            }

            message.channel.send('Promote roles set.');
        });

        return true;
    }
};
