# About

A simple and sophisticated Twitch bot in Rust.

# Commands

| Command | Args | Description |
| ---     | ---  | ---         |
| ping    | None | responds with "pong" |
| explain | [error code: str] | tries to respond with error in assets/explanations |
| markov  | [start word: str] [word count: int]

# Get started

1. `git clone https://github.com/dynamo58/twitch-bot`
2. rename `.env.example` to `.env` and enter your information (you get it [here](https://chatterino.com/client_login))
3. tweak your config in `assets/config.json`
4. create a blank `db.db` file in the root
5. everything set up, you can do `cargo run` or something
