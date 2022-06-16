use dotenv::dotenv;
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
use serenity::model::prelude::Ready;
use serenity::prelude::{Context, EventHandler, GatewayIntents};
use serenity::utils::Colour;
use serenity::Client;
use serenity::Error;
use std::env;

#[group]
struct General;

struct Handler;

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

async fn respond(ctx: Context, command: &ApplicationCommandInteraction, content: &String) {
    if let Err(why) = command
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
        .await
    {
        println!("Cannot respond to slash command: {}", why);
    }
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

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let squad_command = register_squad_command(ctx).await;
        println!("Pushed global slash command: {:#?}", squad_command);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = parse_squad_command(&command).await;
            respond(ctx, &command, &content).await;
        }
    }
}

#[tokio::main]
async fn main() {
    // Build framework
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
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

    // Start
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
