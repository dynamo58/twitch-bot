# About

A simple sophisticated Twitch bot in Rust.

## Underlying technologies
- Rust
	- twitch-irc
		- used as a twitch irc client
	- sqlx
		- used as a SQLite database driver

# Commands

| Command        | Args                                         | Description                                                        | Required status
| ---            | ---                                          | ---                                                                | ---
| \[prefix\]     | [alias name: str]                            | execute an alias                                                   | None
| accage         | [name: opt(str)]                             | get the account age of spec. user or one's self                    | None
| bench          | Command                                      | measure how long a command takes to execute                        | None
| followage      | [user: opt(str)] [channel: opt(str)]         | get the amount of time a user has been following a channel         | None
| clearreminders | None                                         | clears all reminders the user has set (that are still pending)     | None
| define         | [word: str]                                  | queries a dictionary API for a word definition                     | None
| echo           | [text]                                       | repeats user's message                                             | None
| explain        | [error code: str]                            | tries to respond with error in assets/explanations                 | None
| first          | [nick: opt(str)] [channel: opt(str)]         | get the first logged message of a user (in any channel)            | None
| lurk           | None                                         | go into lurk mode (gets removed upon next message)                 | None
| markov         | [start: str] [count: int]                    | responds with a markov chain generated from saved chat messages    | None
| newcmd         | [type: templ\|paste\|incr] [expression: str] | create a new channel command                                       | Broadcaster / Moderator / VIP         
| offlinetime    | [name: opt(str)]                             | returns the time a user has thus far spent in offline chat         | None
| ping           | None                                         | responds with "pong"                                               | None
| remind         | (xh,xm) [user: str] [text]                   | reminds user when he types if spec. amouunt of time has passed     | None
| remindme       | (xh,xm) [text]                               | shortcut for reminding one's self                                  | None
| rose           | None                                         | send a rose to a random fellow chatter!                            | None
| rmalias        | [alias name: str]                            | remove an alias                                                    | None
| rmrm           | None                                         | alias for the `clearreminders` command                             | None
| say            | [text]                                       | alias for `echo`                                                   | None
| setalias       | [name: str] [cmd expression]                 | set an alias for caller (like a substitue for specificied command) | None
| urban          | [term: text]                                 | queries urbandictionary for a phrase                               | None
| weather        | [location: text]                             | get weather report from specified location                         | None
| wiki           | [phrase: text]                               | tries to query Wikipedia for searched topic/title                  | None
<!-- | translate      | (from,to) [text]                     | translate some text |  -->

# Run yourself

1. `git clone https://github.com/dynamo58/twitch-bot`
2. rename `.env.example` to `.env` and enter your information (you can get it [here](https://chatterino.com/client_login))
3. tweak your config in `assets/config.json`
4. create a blank `db.db` file in the root
5. everything set up, you can do `cargo run` or something

# Credits

This bot is heavily inspired by other Twitch bots, takes some of their features and in some cases tries to build on top of them. Those are, most notably,

- [Supibot](https://github.com/Supinic/supibot) made by [Supinic](https://www.twitch.tv/supinic)
- [kbot](https://github.com/KUNszg/kbot) made by [KUNszg](https://kunszg.com/)