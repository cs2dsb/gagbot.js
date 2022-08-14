/**
 * Abstract template for a bot Command + functionality for loading & detecting commands
 *
 * @author Kay <kylrs00@gmail.com>
 * @license ISC - For more information, see the LICENSE.md file packaged with this file.
 * @since r20.1.0
 * @version v1.4.1
 */

const fs = require('fs');
const path = require('path');
const { Collection, EmbedBuilder } = require('discord.js');
const ArgumentList = require('./ArgumentList.js');
const { checkUserCanExecuteCommand } = require('../Permissions');
const { str } = require('./arguments.js');

module.exports = class Command {

    static #DEFAULT_OPTIONS = {
        allowLeadingWhitespace : true,
    };

    /**
     * Load commands from the filesystem
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.1.0
     *
     * @param {Client} client
     * @param {string} commandsDir
     */
    static loadCommands(client, commandsDir) {

        if (!fs.existsSync(commandsDir) || !fs.lstatSync(commandsDir).isDirectory()) return;

        if (!client.hasOwnProperty('commands')) client.commands = new Collection();

        fs.readdirSync(commandsDir)
            .filter((file) => file.endsWith('.js'))
            .forEach((file) => {
                const commandPath = path.resolve(path.join(commandsDir, file));
                const commandClass = require(commandPath);
                const command = new commandClass();
                client.commands.set(command.name, command);

                console.log(`  + command ${command.name}`);
            });
    }


    /**
     * Take a Message and check if a command has been sent. If it has, execute it.
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.1.0
     *
     * @param {Client} client
     * @param {Message} message
     * @param {object} options
     * @returns {Error|undefined}
     */
    static async dispatchCommand(client, message, options) {
        if (!client.hasOwnProperty('commands')) return;

        options = Object.assign(Command.#DEFAULT_OPTIONS, options || {});

        // Match the guild's prefix, or the bot's tag, otherwise ignore the message
        const regexMention = `<@!?${client.user.id}>`;
        const gid = message.guild.id;
        const prefix = client.prefixes[gid];
        const regexPrefix = prefix.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

        const summoner = new RegExp(`^((${regexMention})|(${regexPrefix}))`);
        const matches = message.content.match(summoner);
        if (!matches) return;

        let tail = message.content.substring(matches[0].length);
        if (options.allowLeadingWhitespace) {
            tail = tail.trimStart();
        }
        if (tail.length === 0) return;

        // Parse the name of the command
        let name = tail.split(/\s+/)[0];
        tail = tail.substring(name.length).trimStart();

        // Get the command from the client's Collection, if it exists
        if (!client.commands.has(name)) return;
        let command = client.commands.get(name);

        // If the command doesn't exist, fail silently
        if (!(await checkUserCanExecuteCommand(message.guild, message.author, command))) return;

        let error = null;

        // Parse the arguments
        let args = command.parseArgs(tail);

        // If the arguments are not invalid
        if (args instanceof Error) error = args;
        else if (!(await command.execute(client, message, args))) error = new Error(`Usage Error`);

        if (error) {
            message.channel.send({ embeds: [new EmbedBuilder()
                .setTitle(error.message)
                .addFields(
                    { name: 'Usage', value: '`' + prefix + command.getUsage() + '`'},
                    { name: 'Description', value: command.description },
                )
                .setColor(0xff0000)
                .setThumbnail(`https://cdn.discordapp.com/emojis/708352247804854285.png`)]});
        }
    }


    /**
     * Command constructor. Prevent construction of abstract class.
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.1.0
     */
    constructor(name, description, permissionNode, permissionDefault, args) {
        if (new.target === Command) {
            throw new TypeError("Cannot construct Abstract instances directly");
        }

        this.name = name;
        this.description = description;
        this.permissionNode = permissionNode;
        this.permissionDefault = permissionDefault;
        this.args = args;
    }

    /**
     * Parse an ArgumentList from a string using the args pattern from the command definition
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.1.0
     *
     * @param {string} tail
     * @returns {Error|ArgumentList}
     */
    parseArgs(tail) {
        let args = new ArgumentList();
        // Only parse args if they are required
        if (this.args instanceof Object) {

            // If required args specify keys use them, else use numbers [0, n)
            const names = Object.keys(this.args);

            // Iterate over the required arguments, attempting to match a string that can be parsed as the given type
            for (let name of names) {
                const type = this.args[name];
                const [match, rest] = type(tail);

                if (match === null) {
                    const next = tail.length > 0 ? tail.split(/\s+/)[0] : 'END';
                    return new Error(`Expected \`${name}\`:\`${type.name}\`, found '${next}'.`);
                }

                args.add(name, match, type);
                tail = rest;
            }

            if (tail.length > 0) return new Error(`Too many arguments!`);
        } else {
            // If args is truthy, but there is nothing to parse, throw an error
            if (this.args && tail.length === 0) return new Error(`Command '${this.name}' can't be called without args.`);

            // Split by whitespace and assume all tokens are of type String
            tail.split(/\s+/)
                .forEach((arg, i) => args.add(i, arg, str));
        }

        return args;
    }

    /**
     * Execute the functionality of the command, given the specified args
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.2.0
     *
     * @returns {string}
     */
    getUsage() {
        let argString = '';

        Object.keys(this.args).forEach((key) => {
            argString += ' ';
            if (!/^\d+$/.test(key)) argString += key + ':';
            const type = this.args[key];
            argString += type.name;
        });

        return this.name + argString;
    }

    /**
     * Execute the functionality of the command, given the specified args
     *
     * @author Kay <kylrs00@gmail.com>
     * @since r20.1.0
     *
     * @param {Client} client
     * @param {Message} message
     * @param {ArgumentList} args
     * @returns {Error|boolean}
     */
    execute(client, message, args) {
        return new Error('Cannot call the abstract Command.');
    }

};
