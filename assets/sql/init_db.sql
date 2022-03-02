CREATE TABLE IF NOT EXISTS user_reminders (
	id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    raise_timestamp TEXT NOT NULL
);