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

fn squad_id(message_id: &String) -> String {
    format!("squad:{}", message_id)
}

fn members_id(message_id: &String) -> String {
    format!("members:{}", message_id)
}

fn member_id(message_id: &String, user_id: &String) -> String {
    format!("member:{}:{}", message_id, user_id)
}

pub fn build_squad(
    con: &mut redis::Connection,
    channel_id: &String,
    message_id: &String,
    size: &String,
) -> redis::RedisResult<()> {
    let squad_id = squad_id(&message_id);
    let members_id = members_id(&message_id);
    let capacity: u8 = size.parse().unwrap();
    redis::cmd("HSET")
        .arg(&squad_id)
        .arg("members")
        .arg(members_id)
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
        .arg("notified")
        .arg(0)
        .query(con)?;
    redis::cmd("EXPIRE")
        .arg(&squad_id)
        .arg(5 * 60 * 60)
        .query(con)?;
    Ok(())
}

pub fn add_member(
    con: &mut redis::Connection,
    message_id: &String,
    user_id: &String,
    expires: u32,
) -> redis::RedisResult<()> {
    let squad_id = squad_id(&message_id);
    let members_id = members_id(&message_id);
    let member_id = member_id(&message_id, &user_id);
    let member_count: u8 = redis::cmd("SCARD").arg(&members_id).query(con).unwrap();
    if member_count == 0 {
        let ttl: u32 = redis::cmd("TTL").arg(&squad_id).query(con).unwrap();
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

pub fn get_members_of(
    con: &mut redis::Connection,
    squad_id: &String,
) -> redis::RedisResult<Vec<UserId>> {
    let members_id = redis::cmd("HGET")
        .arg(&squad_id)
        .arg("members")
        .query::<String>(con)?;
    let redis_members: Vec<String> = redis::cmd("SMEMBERS")
        .arg(&members_id)
        .clone()
        .iter::<String>(con)?
        .collect();
    let mut members = Vec::new();
    for member in redis_members {
        let user_id: UserId = redis::cmd("GET").arg(&member).query::<u64>(con)?.into();
        members.push(user_id);
    }
    Ok(members)
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

pub fn get_full_squads(
    con: &mut redis::Connection
) -> redis::RedisResult<Vec<String>> {
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
        let squad_size = redis::cmd("SCARD")
            .arg(&members_id)
            .query::<u8>(con)?;
        let capacity = redis::cmd("HGET").arg(&squad).arg("capacity").query::<u8>(con)?;
        if squad_size >= capacity {
            full_squads.push(squad.clone());
        }
    }
    Ok(full_squads)
}
