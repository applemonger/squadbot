use crate::embed;
use crate::redis_core;
use serenity::model::interactions::application_command::{
    ApplicationCommand, ApplicationCommandInteraction,
    ApplicationCommandInteractionDataOptionValue, ApplicationCommandOptionType,
};
use serenity::model::interactions::InteractionResponseType;
use serenity::model::prelude::Message;
use serenity::prelude::Context;
use serenity::Error;

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
    capacity: &String,
) -> Result<Message, Error> {
    let description = embed::create_description(&capacity);
    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| embed::build_embed(m, description))
        })
        .await?;
    command.get_interaction_response(&ctx.http).await
}

pub async fn register_squad_command(ctx: Context) -> Result<ApplicationCommand, Error> {
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

pub async fn handle_squad_command(ctx: &Context, command: &ApplicationCommandInteraction) {
    let capacity = parse_squad_command(&command).await;
    let command_result = respond_squad_command(&ctx, &command, &capacity).await;
    let channel_id = command.channel_id.as_u64().to_string();
    match command_result {
        Ok(response) => {
            let mut con = redis_core::get_redis_connection(&ctx).await;
            let message_id = response.id.as_u64().to_string();
            redis_core::build_squad(&mut con, &channel_id, &message_id, &capacity).unwrap();
        }
        Err(_) => {
            println!("Unable to respond to command.");
        }
    }
}
