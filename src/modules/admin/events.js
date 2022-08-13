/**
 * Define `admin` module events
 *
 * @author Kay <kylrs00@gmail.com>
 * @license ISC - For more information, see the LICENSE.md file packaged with this file.
 * @since r20.2.0
 * @version v1.0.0
 */

module.exports = {

    /**
     * Init the ActivityLog collection on startup
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     *
     * @param {Client} client
     */
    async on_ready(client) {
        client.db.activityLog = require('./ActivityLog.js');
    },

    /**
     * Update user's lastMessageTimestamp when a user sends a message
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     *
     * @param {Client} client
     * @param {Message} message
     */
    async on_message(client, message) {
        const query = {
            guild: message.guild.id,
            user: message.author.id,
        };
        const value = {
            $set: {
                lastMessageID: message.id,
                lastMessageTimestamp: message.createdTimestamp,
            },
            $inc: {}
        };
        value["$inc"][`channel_message_counts.${message.channel.id}`] = 1;

        // console.log(query, value);
        client.db.activityLog.updateOne(
            query,
            value,
            { upsert: true, useFindAndModify: false, new: true }
        ).catch((err) => {
            console.error(err);
        }).then((result) => {
            // console.log(result);
        });
    },
};
