use chrono::Utc;
use dotenv::dotenv;
use serenity::async_trait;
use serenity::model::channel::Embed;
use serenity::model::channel::Message;
use serenity::model::channel::Reaction;
use serenity::model::prelude::ReactionType;
use serenity::prelude::*;
use std::env;
pub mod squad;
use crate::squad::{Member, Squad};
use std::collections::HashMap;
use std::str::FromStr;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!squad" {
            let posting = msg
                .channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.title("Assemble your squad!");
                        e.description(
                            "✅ React to this message to ready up!\n\
                            1️⃣ Use the number reacts to indicate for how many hours you are \
                            available.\n\n\
                            SquadBot will message you when at least <size> people are \
                            ready.\n\n",
                        );
                        return e;
                    });
                    return m;
                })
                .await
                .unwrap();
            let member = Member {
                user: &msg.author,
                hours: 5,
                expires: Utc::now(),
            };
            let mut members = vec![member];
            let mut options = HashMap::new();
            options.insert(ReactionType::from_str("1️⃣").unwrap(), 1);
            let squad = Squad {
                posting: posting,
                size: 5,
                expires: Utc::now(),
                channel: &msg.channel_id,
                members: members,
                options: options,
            };
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("TOKEN").expect("token");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Client creation failed.");
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
