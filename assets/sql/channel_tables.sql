CREATE TABLE IF NOT EXISTS CHANNEL_{{ CHANNEL_ID }} (
	id          INTEGER PRIMARY KEY,
	sender_id   INTEGER NOT NULL,
	sender_nick TEXT NOT NULL,
	badges      TEXT,
	timestamp   TEXT NOT NULL,
	message     TEXT
);

CREATE TABLE IF NOT EXISTS CHANNEl_{{ CHANNEL_ID }}_MARKOV (
	id INTEGER PRIMARY KEY,
	word TEXT NOT NULL,
	succ TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS CHANNEL_{{ CHANNEL_ID }}_OFFLINERS (
	id          INTEGER PRIMARY KEY,
	offliner_id INTEGER NOT NULL UNIQUE,
	time_s      INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS CHANNEL_{{ CHANNEL_ID }}_COMMANDS (
	id          INTEGER PRIMARY KEY,
	name        TEXT NOT NULL UNIQUE,
	type        TEXT NOT NULL UNIQUE,
	expression  TEXT NOT NULL,
	metadata    INTEGER DEFAULT 0
);
