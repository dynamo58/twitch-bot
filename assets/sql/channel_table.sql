CREATE TABLE IF NOT EXISTS {{ TABLE_NAME }} (
	id          INTEGER PRIMARY KEY,
	sender_id   INTEGER NOT NULL,
	sender_nick TEXT NOT NULL,
	badges      TEXT,
	timestamp   TEXT NOT NULL
)