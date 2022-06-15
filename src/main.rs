use chrono::Utc;
use dotenv::dotenv;
use serenity::async_trait;
use serenity::model::channel::Message;
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
            // Create new squad posting
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
                .await;
            // Check for errors in creating squad posting
            let posting = match posting {
                Ok(p) => p,
                Err(why) => {
                    println!("Error creating posting: {:?}", why);
                    return;
                }
            };
            // Create first member of the squad
            let member = Member {
                user: &msg.author,
                hours: 5,
                expires: Utc::now(),
            };
            // Add member to members list
            let mut members = vec![member];
            // Create reaction options and corresponding values
            let mut options = HashMap::new();
            options.insert(ReactionType::from_str("1️⃣").unwrap(), 1);
            options.insert(ReactionType::from_str("2️⃣").unwrap(), 2);
            options.insert(ReactionType::from_str("3️⃣").unwrap(), 3);
            options.insert(ReactionType::from_str("4️⃣").unwrap(), 4);
            options.insert(ReactionType::from_str("5️⃣").unwrap(), 5);
            options.insert(ReactionType::from_str("6️⃣").unwrap(), 6);
            options.insert(ReactionType::from_str("7️⃣").unwrap(), 7);
            options.insert(ReactionType::from_str("8️⃣").unwrap(), 8);
            options.insert(ReactionType::from_str("9️⃣").unwrap(), 9);
            // Create new squad
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
