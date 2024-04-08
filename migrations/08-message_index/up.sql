-- Find the bot message_ids
CREATE TEMP TABLE bot_message_ids as
SELECT DISTINCT message_id 
FROM message_log 
WHERE json_extract(message_json, '$.author.bot') = 1;
 
-- Delete any message that matches a bot message_id
DELETE FROM message_log
WHERE message_id IN (SELECT message_id FROM bot_message_ids);

-- Clean up the temp table 
DROP TABLE bot_message_ids;

-- Delete any duplicate message_log entries 
-- Having messages with exactly the same id and timestamp with different json seems like a
-- discord bug/eventual consistency issue. The one example of it in the existing data is all
-- identical except one of the @ed people's avatar URLs was different
DELETE FROM message_log WHERE rowid NOT IN (
	SELECT min(rowid) FROM message_log
	GROUP BY message_id, timestamp, type);

-- Chunks are compressed blobs of messages
CREATE TABLE message_chunk (
    chunk_id INTEGER PRIMARY KEY,
    start_message_index_id INTEGER NOT NULL,
    end_message_index_id INTEGER NOT NULL,
    data BLOB NOT NULL
) STRICT; 

-- The index is used to work out if we have anything for a message_id and where it is
CREATE TABLE message_index (
    message_index_id INTEGER PRIMARY KEY,
    message_id INTEGER NOT NULL, -- Snowflake/u64 --
    
    timestamp TEXT NOT NULL,

    type TEXT NOT NULL
        CHECK(type IN ('CREATE', 'EDIT', 'DELETE', 'PURGE')),

    -- NULL indicates it is in message_chunk_temp currently
    chunk_id INTEGER,
    FOREIGN KEY(chunk_id) REFERENCES message_chunk(chunk_id)
) STRICT;

-- Temporary storage for messages before compression
CREATE TABLE message_chunk_temp (
    message_index_id INTEGER NOT NULL UNIQUE,
    message_json TEXT NOT NULL,
    FOREIGN KEY(message_index_id) REFERENCES message_index(message_index_id)
) STRICT;