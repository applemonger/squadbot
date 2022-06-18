use dotenv::dotenv;
use serenity::async_trait;
use serenity::framework::standard::macros::group;
use serenity::framework::standard::StandardFramework;
use serenity::model::interactions::application_command::{
    ApplicationCommand, ApplicationCommandInteraction,
    ApplicationCommandInteractionDataOptionValue, ApplicationCommandOptionType,
};
use serenity::model::interactions::{Interaction, InteractionResponseType};
use serenity::model::prelude::message_component::MessageComponentInteraction;
use serenity::model::prelude::{Message, Ready};
use serenity::prelude::{Context, EventHandler, GatewayIntents};
use serenity::utils::Colour;
use serenity::Client;
use serenity::Error;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
mod embed;
mod redis_core;

#[group]
struct General;

struct Handler;

async fn parse_squad_command(command: &ApplicationCommandInteraction) -> String {
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

async fn respond_squad_command(
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
                        e.description(embed::create_description(&content));
                        e.colour(Colour::from_rgb(59, 165, 93));
                        return e;
                    });
                    m.components(|c| c.add_action_row(embed::action_row()));
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

async fn handle_squad_command(ctx: &Context, command: &ApplicationCommandInteraction) {
    let capacity = parse_squad_command(&command).await;
    let command_result = respond_squad_command(&ctx, &command, &capacity).await;
    match command_result {
        Ok(response) => {
            let mut con = redis_core::get_redis_connection(&ctx).await;
            let message_id = response.id.as_u64().to_string();
            redis_core::build_squad(&mut con, &message_id, &capacity).unwrap();
        }
        Err(_) => {
            println!("Unable to respond to command.");
        }
    }
}

async fn handle_add_member(ctx: &Context, interaction: &MessageComponentInteraction, expires: u8) {
    let message_id = interaction.message.id.as_u64().to_string();
    let user_id = interaction.user.id.as_u64().to_string();
    let seconds: u32 = u32::from(expires) * 60 * 60;
    let mut con = redis_core::get_redis_connection(&ctx).await;
    redis_core::add_member(&mut con, &message_id, &user_id, seconds).unwrap();
}

fn parse_message_component_interaction_id(
    interaction: &MessageComponentInteraction,
) -> embed::ButtonChoice {
    let id = interaction.data.custom_id.clone();
    match id.parse() {
        Ok(expires) => embed::ButtonChoice::Hours(expires),
        Err(_) => embed::ButtonChoice::Other(id),
    }
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
            Interaction::ApplicationCommand(command) => match command.data.name.as_str() {
                "squad" => {
                    handle_squad_command(&ctx, &command).await;
                }
                _ => {
                    println!("Not implemented.");
                }
            },
            Interaction::MessageComponent(component_interaction) => {
                match parse_message_component_interaction_id(&component_interaction) {
                    embed::ButtonChoice::Hours(expires) => {
                        handle_add_member(&ctx, &component_interaction, expires).await;
                    }
                    embed::ButtonChoice::Other(other) => {
                        println!("{}", other);
                    }
                }
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
        data.insert::<redis_core::Redis>(Arc::new(RwLock::new(redis)));
    }

    // Start
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
