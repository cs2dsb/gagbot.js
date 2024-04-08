use std::{collections::HashMap, io::{Cursor, Read, Write}, str};

use poise::serenity_prelude::{Message, Timestamp};
use rusqlite::{
    params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef},
    Connection, ToSql,
};
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use zstd::{Decoder, Encoder};

use crate::{db::{CompressionState, COMPRESSION_LEVEL, MESSAGE_LOG_CHUNK_SIZE}, ensure, Error, ErrorContext, MessageId};

#[derive(Debug, PartialEq)]
pub enum LogType {
    Create,
    Edit,
    Delete,
    // Purge is the same as delete but exists so the log printed in discord is clear
    Purge,
}

#[derive(Debug)]
pub struct MessageLog {
    pub message_index_id: u64,
    pub message_id: MessageId,
    pub timestamp: Timestamp,
    pub type_: LogType,
    pub message: Option<Message>,
}

impl ToSql for LogType {
    fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
        Ok(match self {
            LogType::Create => ToSqlOutput::Borrowed("CREATE".into()),
            LogType::Edit => ToSqlOutput::Borrowed("EDIT".into()),
            LogType::Delete => ToSqlOutput::Borrowed("DELETE".into()),
            LogType::Purge => ToSqlOutput::Borrowed("PURGE".into()),
        })
    }
}

impl FromSql for LogType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        if let ValueRef::Text(v) = value {
            match str::from_utf8(v).map_err(|e| FromSqlError::Other(Box::new(e)))? {
                "CREATE" => Ok(LogType::Create),
                "EDIT" => Ok(LogType::Edit),
                "DELETE" => Ok(LogType::Delete),
                "PURGE" => Ok(LogType::Purge),
                e => {
                    error!("Unexpected enum variant {} for LogType", e);
                    Err(FromSqlError::InvalidType)
                }
            }
        } else {
            Err(FromSqlError::InvalidType)
        }
    }
}

pub fn log(
    db: &mut Connection,
    message_id: MessageId,
    timestamp: Timestamp,
    type_: LogType,
    message: Option<Message>,
) -> Result<(), Error> {
    // TODO: This is kinda magic behaviour
    if type_ == LogType::Delete {
        // Only log 1 delete, there's some funkyness around bulk deletes
        if db.prepare_cached(
                "SELECT 1 FROM message_index
                WHERE message_id = ?1 AND type IN (?2, ?3)
                LIMIT 1",
            )?.exists(params![message_id, LogType::Delete, LogType::Purge])?
        // Only log if it was created or edited
        || !db.prepare_cached(
                "SELECT 1 FROM message_index
                WHERE message_id = ?1 AND type IN (?2, ?3)
                LIMIT 1"
            )?
            .exists(params![message_id, LogType::Create, LogType::Edit])?
        {
            return Ok(());
        }
    }

    if (type_ == LogType::Create || type_ == LogType::Edit) && message.is_none() {
        error!("None message provided to Create or Edit log type")
    }

    let tx = db.transaction()?;
    {   
        let mut stmt = tx.prepare_cached(
            "INSERT INTO message_index (message_id, timestamp, type)
            VALUES(?1, ?2, ?3)",
        )?;
        
        let message_index_id = stmt.insert(params![
            message_id,
            &timestamp.to_rfc3339(),
            type_,
        ])?;
            
        debug!(message_index_id, "message_index inserted");

        if let Some(message) = message {
            let json = serde_json::to_value(message)?;
            
            let mut stmt = tx.prepare_cached("
                INSERT INTO message_chunk_temp (message_index_id, message_json)
                VALUES (?1, ?2)
            ")?;

            stmt.execute(params![message_index_id, json])?;
        }
    }
    tx.commit()?;   
    Ok(())
}
        
pub fn get(
    db: &Connection,
    message_id: MessageId,
) -> Result<Vec<MessageLog>, Error> {
    let mut stmt = db.prepare_cached(
        "SELECT message_index_id, message_id, timestamp, type, chunk_id FROM message_index
        WHERE message_id = ?1
        ORDER BY timestamp DESC",
    )?;

    let mut messages_result: Vec<(MessageLog, Option<u64>)> = stmt
        .query_map(params![message_id], |r| {
            Ok((MessageLog {
                message_index_id: r.get::<_, u64>(0)?,
                message_id: MessageId::from(r.get::<_, u64>(1)?),
                timestamp: Timestamp::from(r.get::<_, String>(2)?),
                type_: r.get(3)?,
                message: None, 
            }, r.get::<_, Option<u64>>(4)?))
        })?
        .collect::<Result<_, _>>()?;

    let mut needs_chunk: HashMap<u64, Vec<(u64, usize)>> = HashMap::new();
    for (i, (m, chunk_id)) in messages_result
        .iter_mut()
        .enumerate()
        .filter(|(_, (m, _))| match m.type_ {
            LogType::Create | LogType::Edit => true,
            _ => false,
        })
    {
        if let Some(chunk_id) = chunk_id {
            if !needs_chunk.contains_key(&chunk_id) {
                needs_chunk.insert(*chunk_id, Vec::new());
            }
            needs_chunk
                .get_mut(chunk_id)
                .unwrap()
                .push((m.message_index_id, i));
        } else {
            m.message = get_message_body(db, m.message_index_id, None)?;
            ensure!(m.message.is_some(), "Failed to get message body for a Create or Edit log entry: message_index_id: {}, chunk_id: {:?}", m.message_index_id, chunk_id);
        }
    }

    for (chunk_id, messages) in needs_chunk.into_iter() {
        let message_index_ids = messages
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();

        let results = decompress_message_body(db, message_index_ids, chunk_id)?;
        ensure!(results.len() == messages.len(), "Expected {} message bodies from decompress_message_body but got {}", messages.len(), results.len());

        for (message_index_id, body) in results.into_iter() {
            if let Some((_, i)) = messages.iter().find(|(id, _)| *id == message_index_id) {
                messages_result[*i].0.message = body;
            } else {
                Err(anyhow::anyhow!("message_index_id {} returned from decompress_message_body doesn't match any requested id", message_index_id))?
            }
        }
    }

    Ok(messages_result
        .into_iter()
        .map(|x| x.0)
        .collect())
}

#[instrument(skip(db))]
fn decompress_message_body(db: &Connection, mut message_index_ids: Vec<u64>, chunk_id: u64) -> Result<Vec<(u64, Option<Message>)>, Error> {
    ensure!(message_index_ids.len() > 0, "0 message_index_ids passed into decompress_message_body");

    let mut stmt = db.prepare_cached("
        SELECT chunk_id, start_message_index_id, end_message_index_id, data 
        FROM message_chunk
        WHERE chunk_id = ?1
        LIMIT 1
    ")?;
    let (chunk_id, start_id, end_id, data) = stmt.query_row(params![chunk_id], |r| {
        let chunk_id: u64 = r.get(0)?;
        let start_id: u64 = r.get(1)?;
        let end_id: u64 = r.get(2)?;
        let data: Vec<u8> = r.get(3)?;
        Ok((chunk_id, start_id, end_id, data))
    })?;

    for message_index_id in message_index_ids.iter() {
        ensure!(*message_index_id >= start_id && *message_index_id <= end_id, 
            "message_index_id ({message_index_id}) outside bounds of chunk ({chunk_id}, {start_id} -> {end_id})");
    }

    message_index_ids.sort();

    let mut zstd_buffer = Vec::with_capacity(MESSAGE_LOG_CHUNK_SIZE as usize);
    
    let mut zdec = Decoder::new(Cursor::new(data))?;
    zdec.read_to_end(&mut zstd_buffer)?;
    
    let mut cobs_buffer = Vec::new();
    let mut count = 0;
    let mut start_i = 0;

    let mut results = Vec::new();

    for (end_i, _) in zstd_buffer
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == corncobs::ZERO)
    {
        let index_id = start_id + count;

        // Look for the ids we care about
        if index_id == message_index_ids[0] {
            // Cobs decode the correct range of bytes
            corncobs::decode(&zstd_buffer[start_i..=end_i], &mut cobs_buffer)?;
            
            let message = if cobs_buffer.len() == 0 {
                None
            } else { 
                serde_json::from_slice::<Message>(&cobs_buffer)
                    .context("Decoding compressed chunk into Message")
                    .map(|m| Some(m))?
            };
            let message_index_id = message_index_ids.remove(0);
            results.push((message_index_id, message));

            if message_index_ids.len() == 0 {
                break;
            }
            // Clear the cobs_buffer for reuse
            cobs_buffer.clear();
        }

        count += 1;
        start_i = end_i + 1;
    }

    if message_index_ids.len() > 0 {
        Err(anyhow::anyhow!("Failed to find message_index_id ({:?}) in chunk ({chunk_id}) despite it being within the declared chunk bounds ({start_id} -> {end_id})", message_index_ids))?
    } else {
        Ok(results)
    }

}

fn get_message_body(db: &Connection, message_index_id: u64, chunk_id: Option<u64>) -> Result<Option<Message>, Error> {
    let message = if let Some(chunk_id) = chunk_id {
        let mut r = decompress_message_body(db, vec![message_index_id], chunk_id)?;
        ensure!(r.len() == 1, "Expected 1 message body from decompress_message_body but got {}", r.len());
        r.remove(0).1
    } else {
        let mut stmt = db.prepare_cached("
            SELECT message_json 
            FROM message_chunk_temp 
            WHERE message_index_id = ?1
            LIMIT 1
        ")?;
        stmt.query_row(params![message_index_id], |r| 
            r.get::<_, Option<serde_json::Value>>(0))?
            .map(|v| serde_json::from_value(v))
            .transpose()
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
    };
    Ok(message)
}
 
pub fn verify_compressed_chunks(db: &Connection) -> Result<(), Error> {
    let mut stmt = db.prepare("SELECT message_index_id, message_id, type FROM message_index")?;
    let index_map = stmt.query_map((), |r| Ok((
        r.get::<_, u64>(0)?,
        (
            r.get::<_, u64>(1)?,
            r.get::<_, String>(2)?,
        ),
    )))?
    .collect::<Result<HashMap<_, _>, _>>()?;

    let mut cobs_buffer = Vec::new();
    let mut zstd_buffer = Vec::with_capacity(MESSAGE_LOG_CHUNK_SIZE as usize);
    let mut stmt = db.prepare("SELECT chunk_id, start_message_index_id, end_message_index_id, data FROM message_chunk")?;
    let mut rows = stmt.query(())?;
    
    while let Some(r) = rows.next()? {
        let chunk_id: u64 = r.get(0)?;
        let start_id: u64 = r.get(1)?;
        let end_id: u64 = r.get(2)?;
        let data: Vec<u8> = r.get(3)?;
        
        let mut zdec = Decoder::new(Cursor::new(data))?;
        zstd_buffer.clear();
        let z_len = zdec.read_to_end(&mut zstd_buffer)?;
        assert_eq!(z_len, zstd_buffer.len());

        let mut count = 0;
        let mut start_i = 0;

        for (end_i, _) in zstd_buffer
            .iter()
            .enumerate()
            .filter(|(_, byte)| **byte == corncobs::ZERO)
        {
            assert_ne!(zstd_buffer[start_i], corncobs::ZERO);
            assert_eq!(zstd_buffer[end_i], corncobs::ZERO);
            cobs_buffer.clear();
            corncobs::decode(&zstd_buffer[start_i..=end_i], &mut cobs_buffer)?;
            start_i = end_i + 1;
            
            if cobs_buffer.len() > 0 {
                let index_id = start_id + count;
                let (message_id, type_) = if let Some(mid) = index_map.get(&index_id) {
                    mid.clone()
                } else {
                    error!("Failed to get message_id for message_index_id {}", index_id);
                    (0, "unknown".to_string())
                };
                match serde_json::from_slice::<Message>(&cobs_buffer) {
                    Ok(m) => if m.id != message_id {
                        error!(chunk_id, index_id, message_id, decoded_id = %m.id, type_, "Decoded message_id doesn't match");
                    },
                    Err(e) => error!("Failed to get message from message_index_id {}: {}", index_id, e),
                }
            }

            count += 1;
        }

        let count_target = end_id - start_id + 1;
        if count != count_target {
            error!(chunk_id, count, end_id, start_id, count_target, "Incorrect count in chunk")
        } else {
            // trace!(chunk_id, count, end_id, start_id, count_target, "Chunk OK")
        }
    }
    Ok(())
}

#[instrument(skip(db))]
pub fn get_compression_state(
    db: &Connection,
) -> Result<CompressionState, Error> {
    let (uncompressed_messages, uncompressed_bytes): (u64, u64)  = {
        let mut count_stmt = db.prepare(
            "SELECT count(1), COALESCE(sum(length(message_json)), 0) FROM message_chunk_temp",
        )?;
        count_stmt.query_row((), 
        |r| Ok((
            r.get(0)?, 
            r.get(1)?)))?
    };

    let compressed_messages:u64 = {
        let mut count_stmt = db.prepare(
            "SELECT count(1) FROM message_index WHERE chunk_id IS NOT NULL",
        )?;
        count_stmt.query_row((), 
        |r| Ok(r.get(0)?))?
    };

    let (compressed_bytes, chunks): (u64, u64)  = {
        let mut count_stmt = db.prepare(
            "SELECT COALESCE(sum(length(data)),0), count(1) FROM message_chunk",
        )?;
        count_stmt.query_row((), 
        |r| Ok((
            r.get(0)?, 
            r.get(1)?)))?
    };
    
    Ok(CompressionState {
        uncompressed_messages,
        uncompressed_bytes,
        compressed_messages,
        compressed_bytes,
        chunks,
    })
}

/// Check if there are enough uncompressed messages to fill a compressed chunk. If so,
/// compress them and insert the chunk.
/// 
/// Returns true if there are enough uncompressed messages to create more chunks
#[instrument(skip(db))]
pub fn compress(
    db: &mut Connection,
) -> Result<bool, Error> {
    let (uncompressed_size, start_message_index_id): (u64, u64)  = {
        let mut count_stmt = db.prepare_cached(
            "SELECT COALESCE(sum(length(message_json)), 0), COALESCE(min(message_index_id), 0) FROM message_chunk_temp",
        )?;
        count_stmt.query_row((), 
        |r| Ok((
            r.get(0)?, 
            r.get(1)?)))?
    };

    info!("uncompressed_size: {uncompressed_size}");
    info!("start_message_index_id: {start_message_index_id}");

    if uncompressed_size > MESSAGE_LOG_CHUNK_SIZE {
        let mut message_count = 0;
        let mut dummy_count = 0;
        let mut remaining_bytes = MESSAGE_LOG_CHUNK_SIZE as usize;
        let mut compressed = Vec::<u8>::with_capacity(MESSAGE_LOG_CHUNK_SIZE as usize);
        let mut next_message_index_id = start_message_index_id;
        let mut cobs_buffer = Vec::new();
        let mut encoder = Encoder::new(&mut compressed, COMPRESSION_LEVEL)?;
        
        let _span = span!(Level::DEBUG, "Compressing message chunk").entered();
        
        {
            let mut push = |data: Vec<u8>| {
                message_count += 1;

                // Make sure there's enough space in the buffer
                cobs_buffer.resize_with(corncobs::max_encoded_len(data.len()), Default::default);
                let cobs_len = corncobs::encode_buf(&data, &mut cobs_buffer);
                assert!(cobs_len > 0);

                encoder.write_all(&cobs_buffer[..cobs_len])
            };

            let mut fetch_stmt = db.prepare_cached(
                "SELECT message_index_id, CAST(message_json AS BLOB)
                FROM message_chunk_temp
                WHERE message_index_id >= ?1"
            )?;

            let mut rows = fetch_stmt.query(params![start_message_index_id])?;
            while let Some(r) = rows.next()? {
                let id: u64 = r.get(0)?;

                // There can't be gaps in the chunk so make dummy entries for
                // any missing ids. There shouldn't be any but this check is
                // in case any manual deletes are done in future
                while next_message_index_id < id {
                    warn!("Missing message_index_id {next_message_index_id} in message_chunk_temp");
                    dummy_count += 1;
                    push(Vec::new())?;
                    next_message_index_id += 1;
                }
 
                assert_eq!(next_message_index_id, id);

                let data: Vec<u8> = r.get(1)?;
                if remaining_bytes >= data.len() 
                   // This is to make sure if we have 1 message that is bigger than the desired chunk
                   // we won't get stuck in a loop making no progress
                   || data.len() >= MESSAGE_LOG_CHUNK_SIZE as usize
                {
                    trace!("Adding {id} with length {}", data.len());
                    remaining_bytes -= data.len();
                    push(data)?;
                } else {
                    break;
                }

                next_message_index_id += 1;
            }
        }

        let end_message_index_id = next_message_index_id - 1;

        encoder.finish()?;

        let expected_count = end_message_index_id - start_message_index_id + 1;
        assert_eq!(expected_count, message_count);

        debug!( 
            message_count, 
            dummy_count,
            chunk_used_size=MESSAGE_LOG_CHUNK_SIZE - remaining_bytes as u64, 
            chunk_max_size=MESSAGE_LOG_CHUNK_SIZE,
            compressed_size=compressed.len(),
            start_message_index_id,
            end_message_index_id,
        );

        let tx = db.transaction()?;

        {
            let mut insert_stmt = tx.prepare_cached(
                "INSERT INTO message_chunk (start_message_index_id, end_message_index_id, data)
                VALUES (?1, ?2, ?3)"
            )?;
            let chunk_id = insert_stmt.insert(params![
                start_message_index_id,
                end_message_index_id,
                compressed,
            ])?;
            debug!(chunk_id, "Chunk inserted");
            
            let mut update_stmt = tx.prepare_cached(
                "UPDATE message_index
                SET chunk_id = ?1
                WHERE message_index_id BETWEEN ?2 AND ?3"
            )?;
            update_stmt.execute(params![
                chunk_id,
                start_message_index_id,
                end_message_index_id,
            ])?;

            let mut delete_stmt = tx.prepare_cached(
                "DELETE FROM message_chunk_temp
                WHERE message_index_id BETWEEN ?1 AND ?2"
            )?;
            delete_stmt.execute(params![
                start_message_index_id,
                end_message_index_id,
            ])?;

        }

        tx.commit()?;

        let more = uncompressed_size - remaining_bytes as u64 > MESSAGE_LOG_CHUNK_SIZE;
        Ok(more)
    } else {
        Ok(false)
    }
}
