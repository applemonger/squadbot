use redis;
use serenity::model::id::UserId;
use serenity::prelude::Context;
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
    redis::cmd("HGET")
        .arg(&squad_id)
        .arg("capacity")
        .query(con)
}

pub fn get_members(con: &mut redis::Connection, message_id: &String) -> Vec<UserId> {
    let members_id = members_id(&message_id);
    let mut cmd = redis::cmd("SMEMBERS");
    let redis_members: Vec<String> = cmd
        .arg(&members_id)
        .clone()
        .iter(con)
        .unwrap()
        .map(|x: String| x.to_string())
        .collect();
    let mut members = Vec::new();
    for member in redis_members {
        let user_id: UserId = redis::cmd("GET")
            .arg(member)
            .query::<u64>(con)
            .unwrap()
            .into();
        members.push(user_id);
    }
    members
}
