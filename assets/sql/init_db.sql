CREATE TABLE IF NOT EXISTS user_reminders (
	id INTEGER PRIMARY KEY,
	from_user_name TEXT NOT NULL,
    for_user_name TEXT NOT NULL,
    raise_timestamp TEXT NOT NULL,
	message TEXT NOT NULL
);