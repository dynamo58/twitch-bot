# About

A simple Twitch bot in Rust with quite specific features.

# Commands

| Command        | Args                             | Description          |
| ---            | ---                              | ---                  |
| ping           | None                             | responds with "pong" |
| explain        | [error code: str]                | tries to respond with error in assets/explanations |
| markov         | [start: str] [count: int]        | responds with a markov chain generated from saved chat messages |
| echo           | \[text\]                         | repeats user's message |
| remind         | \(xh,xm\) \[user: str\] \[text\] | remind a  user in a specified amount of time (whenever he types a messages after the specified duration has passed) |
| remindme       | \(xh,xm\) \[text\]               | shortcut for reminding one's self |
| clearreminders | None                             | clears all reminders the user has set (that are still pending)
| rmrm           | None                             | alias for the `clearreminders` command 

# Get started

1. `git clone https://github.com/dynamo58/twitch-bot`
2. rename `.env.example` to `.env` and enter your information (you get it [here](https://chatterino.com/client_login))
3. tweak your config in `assets/config.json`
4. create a blank `db.db` file in the root
5. everything set up, you can do `cargo run` or something
