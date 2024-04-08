-- This is needed because the message_chunk_temp might have stuff deleted from it as
-- part of the compression process so we can't rely on it's max value and we can't 
-- reorder them to select the max for the index first because of foreign key constraints
CREATE TEMP TABLE max_message_index_id AS 
SELECT coalesce(max(message_index_id), 0) AS value
FROM message_index;

-- Create the index first so the foreign key on chunk temp is happy
INSERT INTO message_index (message_index_id, message_id, timestamp, type)
SELECT message_index_id, message_id, timestamp, type 
FROM message_log
WHERE 
	message_log.message_index_id > (SELECT value FROM max_message_index_id LIMIT 1);

-- Grab the message_json and insert into chunk temp
INSERT INTO message_chunk_temp (message_index_id, message_json)
SELECT message_log.message_index_id, message_log.message_json
FROM message_log
WHERE 
	message_log.message_index_id > (SELECT value FROM max_message_index_id LIMIT 1)
	AND message_log.message_json IS NOT NULL;
	
DROP TABLE max_message_index_id;