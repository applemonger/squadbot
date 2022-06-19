use dotenv::dotenv;
use serenity::async_trait;
use serenity::framework::standard::macros::group;
use serenity::framework::standard::StandardFramework;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::model::prelude::Interaction;
use serenity::prelude::{Context, EventHandler, GatewayIntents};
use serenity::Client;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
mod embed;
mod notify;
mod redis_core;
mod squad;

#[group]
struct General;

struct Handler {
    is_loop_running: AtomicBool,
}

const UPDATE_POLL_SECONDS: u64 = 30;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        match squad::register_squad_command(ctx).await {
            Ok(_) => println!("Registered global commands."),
            Err(error) => println!("Error registering global commands: {:?}", error),
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(command) => match command.data.name.as_str() {
                "squad" => {
                    squad::handle_squad_command(&ctx, &command).await;
                }
                _ => {
                    println!("Not implemented.");
                }
            },
            Interaction::MessageComponent(component_interaction) => {
                match squad::parse_component_id(&component_interaction) {
                    embed::ButtonChoice::Hours(expires) => {
                        squad::handle_add_member(&ctx, &component_interaction, expires).await;
                    }
                    embed::ButtonChoice::Leave(_) => {
                        squad::handle_delete_member(&ctx, &component_interaction).await;
                    }
                }
                if let Err(why) = component_interaction.defer(&ctx.http).await {
                    println!("{:?}", why);
                }
            }
            _ => {}
        }
    }

    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        println!("Cache ready.");
        let ctx = Arc::new(ctx);
        if !self.is_loop_running.load(Ordering::Relaxed) {
            let ctx1 = Arc::clone(&ctx);
            tokio::spawn(async move {
                loop {
                    let ctx2 = Arc::clone(&ctx1);
                    let mut con = redis_core::get_redis_connection(&ctx2).await;
                    let postings = redis_core::get_postings(&mut con).unwrap();
                    for (key, value) in &postings {
                        embed::build_message(&ctx2, &value, &mut con, &key.to_string()).await;
                    }
                    let full_squads = redis_core::get_full_squads(&mut con).unwrap();
                    notify::notify_squads(&ctx2, &mut con, full_squads).await;
                    tokio::time::sleep(Duration::from_secs(UPDATE_POLL_SECONDS)).await;
                }
            });
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }
}

#[tokio::main]
async fn main() {
    // Build framework
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&GENERAL_GROUP);

    // Load Discord bot token
    dotenv().ok();
    let token = env::var("TOKEN").expect("token");

    // Set Discord intents
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::MESSAGE_CONTENT;

    // Build client
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
        })
        .framework(framework)
        .await
        .expect("Client creation failed.");

    // Add Redis connection
    {
        let mut data = client.data.write().await;
        let redis = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
        data.insert::<redis_core::Redis>(Arc::new(RwLock::new(redis)));
    }

    // Start
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
