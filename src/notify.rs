use serenity::model::prelude::Mention;
use serenity::client::Context;
use crate::redis_core;

pub async fn notify_squads(ctx: &Context, con: &mut redis::Connection, squads: Vec<String>) {
    for squad in squads {
        let members = redis_core::get_members_of(con, &squad).unwrap();
        let channel = redis_core::get_channel_of(con, &squad).unwrap();
        let channel_mention = format!("{}", Mention::from(channel));
        let mut roster = String::new();
        for member in &members {
            let mention = &format!("{}\n", Mention::from(*member))[..];
            roster.push_str(mention);
        }
        for member in members {
            let dm_channel = member.create_dm_channel(&ctx.http).await.unwrap();
            dm_channel.send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.title("**Your squad is ready!**");
                    e.description(format!("Members:\n{}\n\n{}", roster, &channel_mention));
                    e
                });
                m
            }).await.unwrap();
        }
    }
}

