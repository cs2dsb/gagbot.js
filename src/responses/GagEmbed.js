/**
 * A MessageEmbed with yellow colour and :gagbot: thumbnail
 *
 * @author Kay <kylrs00@gmail.com>
 * @license ISC - For more information, see the LICENSE.md file packaged with this file.
 * @since r20.2.0
 * @version v1.0.0
 */


const { EmbedBuilder } = require('discord.js');

module.exports = class ErrorEmbed extends EmbedBuilder {

    constructor(title, message) {
        super();
        this.setColor(0xEBC634);
        this.setThumbnail('https://cdn.discordapp.com/emojis/708352151558029322.png');
        this.setTitle(title);
        if (message.length > 0) {
            this.setDescription(message);
        }
    }

};
