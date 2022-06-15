use chrono::{DateTime, TimeZone};
use serenity::model::channel::Message;
use serenity::model::channel::ReactionType;
use serenity::model::id::ChannelId;
use serenity::model::user::User;
use serenity::prelude::*;
use std::collections::HashMap;

pub struct Squad<'a, Tz: TimeZone> {
    pub posting: Message,
    pub size: u32,
    pub expires: DateTime<Tz>,
    pub channel: &'a ChannelId,
    pub members: Vec<Member<'a, Tz>>,
    pub options: HashMap<ReactionType, u8>,
}

pub struct Member<'a, Tz: TimeZone> {
    pub user: &'a User,
    pub hours: u8,
    pub expires: DateTime<Tz>,
}
