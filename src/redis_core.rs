use redis;
use serenity::model::id::UserId;
use serenity::model::prelude::{ChannelId, MessageId};
use serenity::prelude::Context;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use typemap_rev::TypeMapKey;

pub struct Redis;

impl TypeMapKey for Redis {
    type Value = Arc<RwLock<redis::Client>>;
}

pub async fn get_redis_connection(ctx: &Context) -> redis::Connection {
    let data_read = ctx.data.read().await;
    let redis_client_lock = data_read
        .get::<Redis>()
        .expect("Expected Redis in TypeMap")
        .clone();
    let redis_client = redis_client_lock.read().await;
    redis_client.get_connection().unwrap()
}

pub fn squad_id(message_id: &String) -> String {
    format!("squad:{}", message_id)
}

fn members_id(message_id: &String) -> String {
    format!("members:{}", message_id)
}

fn member_id(message_id: &String, user_id: &String) -> String {
    format!("member:{}:{}", message_id, user_id)
}

pub fn posting_id(message_id: &String) -> String {
    format!("posting:{}", message_id)
}

pub fn build_squad(
    con: &mut redis::Connection,
    channel_id: &String,
    message_id: &String,
    size: &String,
) -> redis::RedisResult<()> {
    let squad_id = squad_id(&message_id);
    let members_id = members_id(&message_id);
    let posting_id = posting_id(&message_id);
    let capacity: u8 = size.parse().unwrap();
    redis::cmd("SET")
        .arg(&posting_id)
        .arg(&squad_id)
        .arg("EX")
        .arg(10 * 60 * 60)
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
        .arg(24 * 60 * 60)
        .query(con)?;
    Ok(())
}

pub fn add_member(
    con: &mut redis::Connection,
    message_id: &String,
    user_id: &String,
    expires: u32,
) -> redis::RedisResult<()> {
    let members_id = members_id(&message_id);
    let member_id = member_id(&message_id, &user_id);
    let posting_id = posting_id(&message_id);
    let member_count: u8 = redis::cmd("SCARD").arg(&members_id).query(con).unwrap();
    if member_count == 0 {
        let ttl: u32 = redis::cmd("TTL").arg(&posting_id).query(con).unwrap();
        redis::cmd("SADD")
            .arg(&members_id)
            .arg(&member_id)
            .query(con)?;
        redis::cmd("EXPIRE").arg(&members_id).arg(ttl).query(con)?;
    } else {
        let capacity: u8 = get_capacity(con, &message_id).unwrap();
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

    Ok(())
}

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

pub fn get_capacity(con: &mut redis::Connection, message_id: &String) -> redis::RedisResult<u8> {
    let squad_id = squad_id(&message_id);
    redis::cmd("HGET").arg(&squad_id).arg("capacity").query(con)
}

pub fn get_members(
    con: &mut redis::Connection,
    message_id: &String,
) -> redis::RedisResult<HashMap<UserId, u64>> {
    let members_id = members_id(&message_id);
    let redis_members: Vec<String> = redis::cmd("SMEMBERS")
        .arg(&members_id)
        .clone()
        .iter::<String>(con)?
        .collect();
    let mut ttls = HashMap::new();
    for member in redis_members {
        let user_id: UserId = redis::cmd("GET").arg(&member).query::<u64>(con)?.into();
        let ttl: u64 = redis::cmd("TTL").arg(&member).query::<u64>(con)?;
        ttls.insert(user_id, ttl);
    }
    Ok(ttls)
}

pub fn get_ttl(con: &mut redis::Connection, key: &String) -> redis::RedisResult<u64> {
    redis::cmd("TTL").arg(&key).query::<u64>(con)
}

pub fn get_members_of(
    con: &mut redis::Connection,
    squad_id: &String,
) -> redis::RedisResult<HashMap<UserId, u64>> {
    let message_id = redis::cmd("HGET")
        .arg(&squad_id)
        .arg("message")
        .query::<String>(con)?;
    get_members(con, &message_id)
}

pub fn get_channel_of(
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

pub fn fill_squad(con: &mut redis::Connection, squad_id: &String) -> redis::RedisResult<()> {
    redis::cmd("HSET")
        .arg(&squad_id)
        .arg("filled")
        .arg(1)
        .query(con)?;
    Ok(())
}

pub fn get_filled(con: &mut redis::Connection, squad_id: &String) -> redis::RedisResult<u8> {
    redis::cmd("HGET")
        .arg(&squad_id)
        .arg("filled")
        .query::<u8>(con)
}

pub fn get_squad_status(con: &mut redis::Connection, squad_id: &String) -> redis::RedisResult<u8> {
    let posting_id = redis::cmd("HGET")
        .arg(&squad_id)
        .arg("posting")
        .query::<String>(con)?;
    let exists = redis::cmd("EXISTS").arg(&posting_id).query::<u8>(con)?;
    if exists == 0 {
        redis::cmd("DEL").arg(&squad_id).query(con)?;
        return Ok(0);
    } else {
        let filled = get_filled(con, &squad_id).unwrap();
        if filled == 0 {
            return Ok(1);
        } else {
            return Ok(2);
        }
    }
}
