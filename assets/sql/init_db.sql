CREATE TABLE IF NOT EXISTS user_reminders (
	id              INTEGER PRIMARY KEY,
	from_user_id    INTEGER NOT NULL,
	for_user_id     INTEGER NOT NULL,
	raise_timestamp TEXT NOT NULL,
	message         TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS explanations (
	id      INTEGER PRIMARY KEY,
	code    TEXT NOT NULL UNIQUE,
	message TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_aliases (
	id        INTEGER PRIMARY KEY,
	owner_id  INTEGER NOT NULL,
	alias     TEXT NOT NULL,
	alias_cmd TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_feedback (
	id          INTEGER PRIMARY KEY,
	sender_id   INTEGER NOT NULL,
	sender_name INTEGER,
	message     INTEGER NOT NULL,
	time        TEXT NOT NULL
);

INSERT INTO
	explanations (code, message)
	VALUES
		(
			"E0",
			"The command you called generated an error and couldn't be processed. This is most likely due to an internal server error or a possible unhandled exception."
		),
		(
			"E1",
			"The word you tried to create a Markov chain from could not generate one, because it is not yet tracked in the database. Once it appears in the chat, it's gonna get indexed and actually will generate something."
		),
		(
			"E2",
			"You do not have any messages logged so far. Commands do not get saved."
		)
	ON CONFLICT DO NOTHING;