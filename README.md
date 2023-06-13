# gagbot.rs
**GaGBot is a utility bot for discord servers, written in Rust**

To get the latest stable release, check out [the releases page](https://github.com/cs2dsb/gagbot.js/releases).

This is a Rust port of the original [kylrs GaGBot](https://github.com/kylrs/gagbot.js)

## Features

**Got a good idea?** [Open an issue](https://github.com/cs2dsb/gagbot.js/issues) and start the discussion!

## Getting Started
### Prerequisites

### Installation
  1. [Create a Discord Application and get a Bot Token](https://discord.com/developers/docs/intro#bots-and-apps)
      1. `Requires Presence Intent`, `Server Members Intent` and `Message Content Intent` under the `Bot` tab
  2. Invite the bot to your server.
      1. https://discordapi.com/permissions.html to get an invite link without OAuth2
      2. Permissions required
          1. Administrator
  3. Clone the repo or [grab a stable release (recommended)](https://github.com/cs2dsb/gagbot.js/releases)
  4. Install the bot

```
  cd /gagbot.js
  cargo build --release --bin main
```

  5. Create an environment variable named `DISCORD_TOKEN`, and set it to your bot's token.      
  7. Run the bot. If all goes well, you'll see the bot communicating with discord

```
  ./target/release/main
```

  7. You can test your bot using the `/ping` command in your server chat

  8. TODO ~~[Configure GaGBot](https://github.com/kylrs/gagbot.js/wiki/Configuration)!~~

## Built With

## Contributors
 - **daniel_** <cs2dsb@gmail.com> - _Code monkey_
 - **Kay** <kylrs00@gmail.com> - _Mastermind behind the bot_

## License

Free and open source software distributed under the terms of both the [MIT License][lm] and the [Apache License 2.0][la].

[lm]: LICENSE-MIT
[la]: LICENSE-APACHE
