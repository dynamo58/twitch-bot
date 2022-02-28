INSERT INTO {{ TABLE_NAME }} 
	(sender_id, sender_nick, badges, timestamp, message)
VALUES
	(?1, ?2, ?3, ?4, ?5)