use dotenv::dotenv;
use redis;
use serenity::async_trait;
use serenity::builder::{CreateActionRow, CreateButton};
use serenity::framework::standard::macros::group;
use serenity::framework::standard::StandardFramework;
use serenity::model::interactions::application_command::{
    ApplicationCommand, ApplicationCommandInteraction,
    ApplicationCommandInteractionDataOptionValue, ApplicationCommandOptionType,
};
use serenity::model::interactions::message_component::ButtonStyle;
use serenity::model::interactions::{Interaction, InteractionResponseType};
use serenity::model::prelude::{Message, Ready};
use serenity::prelude::{Context, EventHandler, GatewayIntents};
use serenity::utils::Colour;
use serenity::Client;
use serenity::Error;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use typemap_rev::TypeMapKey;

#[group]
struct General;

struct Handler;

struct Redis;

impl TypeMapKey for Redis {
    type Value = Arc<RwLock<redis::Client>>;
}

fn button(hours: u8) -> CreateButton {
    let mut b = CreateButton::default();
    b.custom_id(hours.to_string().to_ascii_lowercase());
    b.label(hours.to_string());
    b.style(ButtonStyle::Primary);
    b
}

fn action_row() -> CreateActionRow {
    let mut ar = CreateActionRow::default();
    // We can add up to 5 buttons per action row
    ar.add_button(button(1));
    ar.add_button(button(2));
    ar.add_button(button(3));
    ar.add_button(button(4));
    ar.add_button(button(5));
    ar
}

async fn parse_squad_command(command: &ApplicationCommandInteraction) -> String {
    match command.data.name.as_str() {
        "squad" => {
            let options = command
                .data
                .options
                .get(0)
                .expect("Expected squad size.")
                .resolved
                .as_ref()
                .expect("Expected integer.");

            if let ApplicationCommandInteractionDataOptionValue::Integer(size) = options {
                size.to_string()
            } else {
                "Please provide a valid size.".to_string()
            }
        }
        _ => "Not implemented :(".to_string(),
    }
}

async fn respond(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    content: &String,
) -> Result<Message, Error> {
    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| {
                    m.embed(|e| {
                        e.title("Assemble your squad!");
                        e.description(format!(
                            "✅ React to this message to ready up!\n\
                                1️⃣ Use the number reacts to indicate for how many hours you are \
                                available.\n\n\
                                SquadBot will message you when at least {} people are \
                                ready.\n\n",
                            content
                        ));
                        e.colour(Colour::from_rgb(59, 165, 93));
                        return e;
                    });
                    m.components(|c| c.add_action_row(action_row()));
                    return m;
                })
        })
        .await?;
    command.get_interaction_response(&ctx.http).await
}

async fn register_squad_command(ctx: Context) -> Result<ApplicationCommand, Error> {
    ApplicationCommand::create_global_application_command(&ctx.http, |command| {
        command
            .name("squad")
            .description("Create a new squad posting")
            .create_option(|option| {
                option
                    .name("size")
                    .description("Number from 2 to 10")
                    .kind(ApplicationCommandOptionType::Integer)
                    .min_int_value(2)
                    .max_int_value(10)
                    .required(true)
            })
    })
    .await
}

fn build_squad(
    con: &mut redis::Connection,
    response: &Message,
    size: &String,
) -> redis::RedisResult<()> {
    let message_id = response.id.as_u64().to_string();
    let squad_id = format!("squad:{}", &message_id);
    let members_id = format!("members:{}", &message_id);
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
    Ok(())
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        match register_squad_command(ctx).await {
            Ok(_) => println!("Registered global commands."),
            Err(error) => println!("Error registering global commands: {:?}", error),
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(command) => {
                let content = parse_squad_command(&command).await;
                match respond(&ctx, &command, &content).await {
                    Ok(response) => {
                        let mut con = {
                            println!("Connecting to redis server...");
                            let data_read = ctx.data.read().await;
                            let redis_client_lock = data_read
                                .get::<Redis>()
                                .expect("Expected Redis in TypeMap")
                                .clone();
                            let redis_client = redis_client_lock.read().await;
                            redis_client.get_connection().unwrap()
                        };
                        if let Err(_) = build_squad(&mut con, &response, &content) {
                            println!("Unable to create squad.");
                        }
                    }
                    Err(_) => {
                        println!("Unable to respond to command.");
                    }
                }
            }
            Interaction::MessageComponent(component_interaction) => {
                println!("{}", component_interaction.message.id);
            }
            _ => {}
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
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::MESSAGE_CONTENT;

    // Build client
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Client creation failed.");

    // Add Redis connection
    {
        let mut data = client.data.write().await;
        let redis = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
        data.insert::<Redis>(Arc::new(RwLock::new(redis)));
    }

    // Start
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
