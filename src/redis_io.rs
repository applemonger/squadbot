use redis;
use serenity::model::id::UserId;
use serenity::model::prelude::{ChannelId, MessageId};
use serenity::prelude::Context;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use typemap_rev::TypeMapKey;
use std::error::Error;

pub struct Redis;

/// Globally available TypeMapKey to store client for Redis connection
impl TypeMapKey for Redis {
    type Value = Arc<RwLock<redis::Client>>;
}

pub enum SquadStatus {
    Expired,
    Forming,
    Filled,
}

/// Expiration time in seconds for squad postings
const POSTING_TTL: u64 = 10 * 60 * 60;
/// Expiration time in seconds for squad data. Squad data is typically deleted through
/// get_squad_status, but this is provided as a safety net to keep the Redis data store
/// clear.
const SQUAD_TTL: u64 = 24 * 60 * 60;

/// Retrieve redis connection from the global data context.
pub async fn get_redis_connection(ctx: &Context) -> Result<redis::Connection, Box<dyn Error>> {
    let data_read = ctx.data.read().await;
    let redis_client_lock = match data_read.get::<Redis>() {
        Some(lock) => lock.clone(),
        None => {
            return Err("Unable to get Redis client lock.".into());
        }
    };
    let redis_client = redis_client_lock.read().await;
    let con = redis_client.get_connection()?;
    Ok(con)
}

/// Helper function to create a squad id for Redis.
/// This is the key of the Hash map which contains the squad data.
pub fn squad_id(message_id: &String) -> String {
    format!("squad:{}", message_id)
}

/// Helper function to create a members id for Redis.
/// This is the key of the Set which contains ids of squad members.
fn members_id(message_id: &String) -> String {
    format!("members:{}", message_id)
}

/// Helper function to create a member id for Redis.
/// This is the Key of the key-value pair for a squad member.
fn member_id(message_id: &String, user_id: &String) -> String {
    format!("member:{}:{}", message_id, user_id)
}

/// Helper function to create a squad posting id.
/// This is the Key of the key-value pair for a squad posting.
pub fn posting_id(message_id: &String) -> String {
    format!("posting:{}", message_id)
}

/// Add new squad data to the Redis data store:
/// HASH squad:msg_id
///     field members: key of Set which contains member ids
///     field posting: key of Key-value pair which contains posting id
///     field channel: id of channel in which squad posting was made
///     field message: id of message containing squad posting
///     field capacity: full size of squad
///     field filled: 0 or 1, whether or not the squad has been filled and notified
///     expires in SQUAD_TTL seconds
/// KEY posting:msg_id
///     contains squad id of the corresponding squad
///     expires in POSTING_TTL seconds
pub fn build_squad(
    con: &mut redis::Connection,
    channel_id: &String,
    message_id: &String,
    capacity: u8,
) -> redis::RedisResult<()> {
    let squad_id = squad_id(&message_id);
    let members_id = members_id(&message_id);
    let posting_id = posting_id(&message_id);
    redis::cmd("SET")
        .arg(&posting_id)
        .arg(&squad_id)
        .arg("EX")
        .arg(POSTING_TTL)
        .query(con)?;
    redis::cmd("HSET")
        .arg(&squad_id)
        .arg("members")
        .arg(members_id)
        .query(con)?;
    redis::cmd("HSET")
        .arg(&squad_id)
        .arg("posting")
        .arg(&posting_id)
        .query(con)?;
    redis::cmd("HSET")
        .arg(&squad_id)
        .arg("channel")
        .arg(&channel_id)
        .query(con)?;
    redis::cmd("HSET")
        .arg(&squad_id)
        .arg("message")
        .arg(&message_id)
        .query(con)?;
    redis::cmd("HSET")
        .arg(&squad_id)
        .arg("capacity")
        .arg(capacity)
        .query(con)?;
    redis::cmd("HSET")
        .arg(&squad_id)
        .arg("filled")
        .arg(0)
        .query(con)?;
    redis::cmd("EXPIRE")
        .arg(&squad_id)
        .arg(SQUAD_TTL)
        .query(con)?;
    Ok(())
}

/// Adds a new member to the corresponding squad in Redis.
/// Creates or appends to ->
/// SET members:msg_id
///     contains member ids of the squad in the form member:msg_id:user_id
///     expires in POSTING_TTL seconds
/// Creates ->
/// KEY member:msg_id:user_id
///     contains Discord user id of squad member
///     expires in <hours * 60 * 60> seconds where hours is chosen from the posting
pub fn add_member(
    con: &mut redis::Connection,
    message_id: &String,
    user_id: &String,
    expires: u32,
) -> redis::RedisResult<()> {
    let members_id = members_id(&message_id);
    let member_id = member_id(&message_id, &user_id);
    let posting_id = posting_id(&message_id);
    let squad_id = squad_id(&message_id);
    let squad_status = get_squad_status(con, &squad_id).unwrap();
    match squad_status {
        SquadStatus::Forming => {
            let member_count: u8 = redis::cmd("SCARD").arg(&members_id).query(con).unwrap();
            if member_count == 0 {
                let ttl: u32 = redis::cmd("TTL").arg(&posting_id).query(con).unwrap();
                redis::cmd("SADD")
                    .arg(&members_id)
                    .arg(&member_id)
                    .query(con)?;
                redis::cmd("EXPIRE").arg(&members_id).arg(ttl).query(con)?;
            } else {
                let capacity: u8 = get_capacity(con, &squad_id).unwrap();
                if member_count < capacity {
                    redis::cmd("SADD")
                        .arg(&members_id)
                        .arg(&member_id)
                        .query(con)?;
                }
            }
            redis::cmd("SET")
                .arg(member_id)
                .arg(user_id)
                .arg("EX")
                .arg(expires)
                .query(con)?;
        }
        _ => {}
    }

    Ok(())
}

/// Deletes a give user from the squad data by removing them from the members Set and
/// deleting the member:msg_id:user_id key-value pair.
pub fn delete_member(
    con: &mut redis::Connection,
    message_id: &String,
    user_id: &String,
) -> redis::RedisResult<()> {
    let members_id = members_id(&message_id);
    let member_id = member_id(&message_id, &user_id);
    redis::cmd("SREM")
        .arg(&members_id)
        .arg(&member_id)
        .query(con)?;
    redis::cmd("DEL").arg(&member_id).query(con)?;

    Ok(())
}

/// Get the capacity from a given squad id
pub fn get_capacity(con: &mut redis::Connection, squad_id: &String) -> redis::RedisResult<u8> {
    redis::cmd("HGET").arg(&squad_id).arg("capacity").query(con)
}

/// Get the members and corresponding expiry times in seconds of a given squad id
/// Also realizes any expired members and deletes them from the members set
pub fn get_members(
    con: &mut redis::Connection,
    squad_id: &String,
) -> redis::RedisResult<HashMap<UserId, u64>> {
    // Check if reference to members id set exists within squad data
    let members_id_field_exists = redis::cmd("HEXISTS")
        .arg(&squad_id)
        .arg("members")
        .query::<u8>(con)?;
    // If it does, get the set key, else early return an empty hashmap
    let members_id = match members_id_field_exists {
        1 => {
            redis::cmd("HGET")
                .arg(&squad_id)
                .arg("members")
                .query::<String>(con)?
        },
        _ => return Ok(HashMap::new()),
    };
    // Check if the members set is populated
    let members_id_populated = redis::cmd("SCARD")
        .arg(&members_id)
        .query::<u8>(con)?;
    // If it doesn't, early return an empty hashmap, else collect member ids
    let redis_members: Vec<String> = match members_id_populated {
        0 => return Ok(HashMap::new()),
        _ => {
            redis::cmd("SMEMBERS")
                .arg(&members_id)
                .clone()
                .iter::<String>(con)?
                .collect()
        }
    };
    // Create hashmap of user ids and corresponding ttls
    let mut members = HashMap::new();
    for member in redis_members {
        let exists = redis::cmd("EXISTS").arg(&member).query::<u8>(con)?;
        if exists == 1 {
            let user_id: UserId = redis::cmd("GET").arg(&member).query::<u64>(con)?.into();
            let ttl: u64 = redis::cmd("TTL").arg(&member).query::<u64>(con)?;
            members.insert(user_id, ttl);
        } else {
            redis::cmd("SREM")
                .arg(&members_id)
                .arg(&member)
                .query(con)?;
        }
    }
    Ok(members)
}

/// Get the time-to-live in seconds of a given key
pub fn get_ttl(con: &mut redis::Connection, key: &String) -> redis::RedisResult<u64> {
    redis::cmd("TTL").arg(&key).query::<u64>(con)
}

/// Get the channel where the squad posting was made for a given squad id
pub fn get_channel(
    con: &mut redis::Connection,
    squad_id: &String,
) -> redis::RedisResult<ChannelId> {
    let channel: ChannelId = redis::cmd("HGET")
        .arg(&squad_id)
        .arg("channel")
        .query::<u64>(con)?
        .into();
    Ok(channel)
}

/// Get the channel and message ids of all current squad postings
pub fn get_postings(
    con: &mut redis::Connection,
) -> redis::RedisResult<HashMap<MessageId, ChannelId>> {
    let squads: Vec<String> = redis::cmd("KEYS")
        .arg("squad:*")
        .clone()
        .iter::<String>(con)?
        .collect();
    let mut postings = HashMap::new();
    for squad in squads {
        let message_id: MessageId = redis::cmd("HGET")
            .arg(&squad)
            .arg("message")
            .query::<u64>(con)?
            .into();
        let channel_id: ChannelId = redis::cmd("HGET")
            .arg(&squad)
            .arg("channel")
            .query::<u64>(con)?
            .into();
        postings.insert(message_id, channel_id);
    }
    Ok(postings)
}

/// Get a list of squad ids of all squads that are currently at capacity and haven't
/// been flagged as filled and notified.
pub fn get_full_squads(con: &mut redis::Connection) -> redis::RedisResult<Vec<String>> {
    let squads: Vec<String> = redis::cmd("KEYS")
        .arg("squad:*")
        .clone()
        .iter::<String>(con)?
        .collect();
    let mut full_squads = Vec::new();
    for squad in squads {
        let members_id = redis::cmd("HGET")
            .arg(&squad)
            .arg("members")
            .query::<String>(con)?;
        let squad_size = redis::cmd("SCARD").arg(&members_id).query::<u8>(con)?;
        let capacity = redis::cmd("HGET")
            .arg(&squad)
            .arg("capacity")
            .query::<u8>(con)?;
        let filled = redis::cmd("HGET")
            .arg(&squad)
            .arg("filled")
            .query::<u8>(con)?;
        if (squad_size >= capacity) && (filled == 0) {
            full_squads.push(squad.clone());
        }
    }
    Ok(full_squads)
}

/// Flag a squad as filled
pub fn fill_squad(con: &mut redis::Connection, squad_id: &String) -> redis::RedisResult<()> {
    redis::cmd("HSET")
        .arg(&squad_id)
        .arg("filled")
        .arg(1)
        .query(con)?;
    Ok(())
}

/// Read the flag indicating whether or not a squad has been filled and notified
pub fn get_filled(con: &mut redis::Connection, squad_id: &String) -> redis::RedisResult<u8> {
    redis::cmd("HGET")
        .arg(&squad_id)
        .arg("filled")
        .query::<u8>(con)
}

/// Get the squad status of a given squad id: Expired, Forming, or Filled
pub fn get_squad_status(
    con: &mut redis::Connection,
    squad_id: &String,
) -> redis::RedisResult<SquadStatus> {
    let posting_id = redis::cmd("HGET")
        .arg(&squad_id)
        .arg("posting")
        .query::<String>(con)?;
    let exists = redis::cmd("EXISTS").arg(&posting_id).query::<u8>(con)?;
    if exists == 0 {
        redis::cmd("DEL").arg(&squad_id).query(con)?;
        return Ok(SquadStatus::Expired);
    } else {
        let filled = get_filled(con, &squad_id).unwrap();
        if filled == 0 {
            return Ok(SquadStatus::Forming);
        } else {
            return Ok(SquadStatus::Filled);
        }
    }
}
