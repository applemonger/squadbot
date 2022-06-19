use crate::embed;
use crate::redis_core;
use serenity::client::Context;
use serenity::model::prelude::Mention;

pub async fn notify_squads(ctx: &Context, con: &mut redis::Connection, squads: Vec<String>) {
    for squad in squads {
        let members = redis_core::get_members_of(con, &squad).unwrap();
        let channel = redis_core::get_channel_of(con, &squad).unwrap();
        let channel_mention = format!("{}", Mention::from(channel));
        let mut roster = String::new();
        for (key, value) in &members {
            let mention = format!("{}", Mention::from(*key));
            let ttl = embed::format_ttl(*value);
            let line = &format!("{} available for {}\n", mention, ttl)[..];
            roster.push_str(line);
        }
        for (user_id, _ttl) in &members {
            let dm_channel = user_id.create_dm_channel(&ctx.http).await.unwrap();
            dm_channel
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.title("**Your squad is ready!**");
                        e.description(format!("Members:\n{}\n\n{}", roster, &channel_mention));
                        e
                    });
                    m
                })
                .await
                .unwrap();
        }
        redis_core::fill_squad(con, &squad).unwrap();
    }
}
