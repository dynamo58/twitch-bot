# About

A simple sophisticated Twitch bot in Rust.

## Underlying technologies
- Rust
	- twitch-irc
		- used as a twitch irc client
	- sqlx
		- used as a SQLite database driver

# Commands

| Command        | Args                                 | Description                                                        |
| ---            | ---                                  | ---                                                                |
| ping           | None                                 | responds with "pong"                                               |
| explain        | [error code: str]                    | tries to respond with error in assets/explanations                 |
| markov         | [start: str] [count: int]            | responds with a markov chain generated from saved chat messages    |
| echo           | [text]                               | repeats user's message                                             |
| say            | [text]                               | alias for `echo`                                                   |
| remind         | (xh,xm) [user: str] [text]           | reminds user when he types if spec. amouunt of time has passed     |
| remindme       | (xh,xm) [text]                       | shortcut for reminding one's self                                  |
| clearreminders | None                                 | clears all reminders the user has set (that are still pending)     |
| rmrm           | None                                 | alias for the `clearreminders` command                             |
| setalias       | [name: str] [cmd expression]         | set an alias for caller (like a substitue for specificied command) |
| \[prefix\]     | [alias name: str]                    | execute an alias                                                   |
| rmalias        | [alias name: str]                    | remove an alias                                                    |
| first          | [nick: opt(str)] [channel: opt(str)] | get the first logged message of a user (in any channel)            |
| rose           | None                                 | send a rose to a random fellow chatter!                            |
| weather        | [location: text]                     | get weather report from specified location                         |
| accage         | [name: opt(str)]                     | get the account age of spec. user or one's self                    |
| lurk           | None                                 | go into lurk mode (gets removed upon next message)                 |
| bench          | Command                              | measure how long a command takes to execute                        |
| offlinetime    | [name: opt(str)]                     | returns the time a user has thus far spent in offline chat         |
| wiki           | [phrase: text]                       | tries to query Wikipedia for searched topic/title                  |
| define         | [word: str]                          | queries a dictionary API for a word definition                     |
<!-- | translate      | (from,to) [text]                     | translate some text |  -->

# Get started

1. `git clone https://github.com/dynamo58/twitch-bot`
2. rename `.env.example` to `.env` and enter your information (you can get it [here](https://chatterino.com/client_login))
3. tweak your config in `assets/config.json`
4. create a blank `db.db` file in the root
5. everything set up, you can do `cargo run` or something

# Credits

This bot is heavily inspired by other Twitch bots, takes some of their features and in some cases tries to build on top of them. Those are, most notably,

- [Supibot](https://github.com/Supinic/supibot) made by [Supinic](https://www.twitch.tv/supinic)
- [kbot](https://github.com/KUNszg/kbot) made by [KUNszg](https://kunszg.com/)