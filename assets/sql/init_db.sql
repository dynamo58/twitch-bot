CREATE TABLE IF NOT EXISTS user_reminders (
	id INTEGER PRIMARY KEY,
	sender_name INTEGER NOT NULL,
    target_user_id INTEGER NOT NULL,
    raise_timestamp TEXT NOT NULL
);