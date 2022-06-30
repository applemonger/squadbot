use crate::embed;
use crate::redis_io;
use serenity::client::Context;
use serenity::model::prelude::Mention;
use std::error::Error;

/// DMs a notification to each member of every given squad (presumably filled squads).
pub async fn notify_squads(
    ctx: &Context,
    con: &mut redis::Connection,
    squads: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    for squad in squads {
        // Get members of squad and the channel of the squad posting
        let members = redis_io::get_members(con, &squad)?;
        // Include roster of squad members in each message
        let mut roster = String::from("**Members**\n");
        for (key, value) in &members {
            let mention = format!("{}", Mention::from(*key));
            let ttl = embed::format_ttl(*value);
            let line = &format!("{} available for {}\n", mention, ttl)[..];
            roster.push_str(line);
        }
        // Build list of channels that the squad was posted in
        let channel_ids = redis_io::get_channels(con, &squad)?;
        let mut channels = String::from("**Channels**\n");
        for channel in &channel_ids {
            let mention = &format!("{}\n", Mention::from(*channel))[..];
            channels.push_str(mention)
        }
        // Mark the squad as filled
        redis_io::fill_squad(con, &squad)?;
        // Send message to each squad member
        for (user_id, _ttl) in &members {
            if let Ok(dm_channel) = user_id.create_dm_channel(&ctx.http).await {
                let _ = dm_channel
                    .send_message(&ctx.http, |m| {
                        m.embed(|e| {
                            e.title("**Your squad is ready!**");
                            e.description(format!("{}\n{}", roster, channels));
                            e
                        });
                        m
                    })
                    .await;
            }
        }
    }
    Ok(())
}
