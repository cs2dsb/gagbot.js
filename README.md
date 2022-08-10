<!--
  @author  Kay <kylrs00@gmail.com>
  @version v1.3.1
-->

# gagbot.js
**GaGBot is a utility bot for discord servers, written in JavaScript for Node.js**

To get the latest stable release, check out [the releases page](https://github.com/kylrs/gagbot.js/releases).

## Features
 - **Module Loader** - GaGBot can dynamically load modules that define new commands and events, making implementing custom features a breeze! _(r20.1.0)_
 - **Permissions** - Fine-tune access to custom commands with simple permission nodes per role. _(r20.1.0)_
 - **Greet Module** - Send new users a welcome message when they join the server. _(r20.2.0)_
 - **Reaction Roles** - Allow users to assign themselves specific roles by reacting to messages. _(r20.2.0)_
### Upcoming Features
 - **Admin Module** - Commands for managing the server, e.g. purging channels, muting users, and so on.
 - **Custom Logging** - Keep track of server activity by choosing what events are logged, and where.
 - **Promotion Tracks** - Allow users to earn roles, with ladders that they can climb automatically.

 **Got a good idea?** [Open an issue](https://github.com/kylrs/gagbot.js/issues) and start the discussion!

 Alternatively, you could contribute to one of these features! Look for open [issues that are awaiting your action](https://github.com/kylrs/gagbot.js/issues?q=is%3Aopen+is%3Aissue+label%3As%3Awaiting).

## Getting Started
### Prerequisites
 - Install `git`
 - Install `node`, version 14.0.0 or later
 - Install `npm`
 - A MongoDB server.

### Installation
  1. [Create a Discord Application and get a Bot Token](https://discord.com/developers/docs/intro#bots-and-apps)
      1. `Requires Presence Intent`, `Server Members Intent` and `Message Content Intent` under the `Bot` tab
  2. Invite the bot to your server.
      1. https://discordapi.com/permissions.html to get an invite link without OAuth2
      2. Permissions required
          1. Administrator
  3. Clone the repo or [grab a stable release (recommended)](https://github.com/kylrs/gagbot.js/releases)
  4. Install the bot

```
  cd /gagbot.js
  npm install
```

  5. Create an environment variable named `DISCORD_TOKEN`, and set it to your bot's token.
      - If you're using the `admin` module, you should also supply the environment variables `PASTEBIN_DEV_KEY`, `PASTEBIN_USER_NAME` and `PASTEBIN_USER_PASSWORD`, containing your Pastebin API developer key, username and password respectively. This is especially necessary for large servers where the `prune` command may select inactive members in excess of the number the bot is able to list in a MessageEmbed.
  6. Add your MongoDB connection string to an environment variable named `MONGO_DB_URI`.
  7. Run the bot. If all goes well, you'll see the modules being loaded, followed by a message that your bot has logged in to Discord.

```
  node src/bot.js
```

  7. You can test your bot using the `ping` command in your server chat, which is included in the `core` module. By default, you can either tag the bot to summon it, or use the prefix `!`.

  8. [Configure GaGBot](https://github.com/kylrs/gagbot.js/wiki/Configuration)!

### Deploying on Heroku

  1. Register and create a new free database on https://cloud.mongodb.com
      1. Select username & password for authentication method
      1. Add 0.0.0.0 to the list of allowed IPs (Heroku IPs change a lot so it's hard/impossible to have a more restricted filter)
      1. Get the connection string from Deployment|Database dashboard (click connect and select "Connect your application"). It looks like `mongodb+srv://<username>:<password>@cluster_.____.mongodb.net/?retryWrites=true&w=majority`
  1. Register on https://www.heroku.com
      1. Create a new app (name doesn't matter, region *probably* doesn't matter, just pick your local region)
      1. Set environment variables
          1. On the `Settings` tab click `Reveal Config Vars`
          1. Create vars for `DISCORD_TOKEN` and `MONGO_DB_URI`. `DISCORD_TOKEN` comes from the https://discord.com/developers/applications under the Bot heading. `MONGO_DB_URI` is the connection string taken from https://cloud.mongodb.com earlier
      1. Configure deployment (`Deploy` tab)
          1. Linking to github is probably easiest
              1. Create a github account
              1. Fork this repository under your own account
              1. Select `GitHub` as the deployment method in Heroku | Deploy tab
                  1. Do the OAuth dance to give Heroku access to your GitHub account
                  1. Select the branch you want to deploy
                      1. `master` contains the latest code but may be broken during development
                      1. `deploy` will only be pushed to once the code is tested so it should be safe to auto deploy from here
                  1. Optionally select automatic deployments if you want Heroku to pick up new versions when you update the selected branch
          1. Heroku git is another option, just follow the instructions on the Heroku | Deploy tab
      1. Once deployment is configured, force at least one deployment
      1. After it's deployed the `Resources` tab should be populated (may need to refresh the browser)
          1. Click the pencil next to `web` and turn it off
          1. Click the pencil next to `worker` and turn it on
          1. It should sort itself out but you can also use `More`|`Restart all dynos`
      1. `More`|`View logs` is useful if something isn't working.
      1. The bot should appear almost immediatly in your server if you already invited it

## Built With

  - [Node.js](https://nodejs.org)
  - [discord.js](https://discord.js.org)
  - [MongoDB](https://www.mongodb.com)

## Versioning

We use MAJOR.MINOR.PATCH semantic versioning in two different flavours. One for versioning code, and the other for releases. For more information, [visit the wiki](https://github.com/kylrs/gagbot.js/wiki/Versioning).

The latest stable release is `r20.1.0`.

## Contributors

 - **Kay** <kylrs00@gmail.com> - _Product Owner_

## License

This code is licensed under the ISC License - see [LICENSE.md](./LICENSE.md) for details.
