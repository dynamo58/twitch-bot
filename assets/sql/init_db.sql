CREATE TABLE IF NOT EXISTS user_reminders (
	id INTEGER PRIMARY KEY,
	from_user_name TEXT NOT NULL,
    for_user_name TEXT NOT NULL,
    raise_timestamp TEXT NOT NULL,
	message TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS explanations (
	id INTEGER PRIMARY KEY,
	code TEXT NOT NULL UNIQUE,
	message TEXT NOT NULL
);

INSERT INTO explanations (code, message) VALUES ("E1", "The word you tried to create a Markov chain from could not generate one, because it is not yet tracked in the database. Once it appears in the chat, it's gonna get indexed and actually will generate something. E1") ON CONFLICT DO NOTHING;