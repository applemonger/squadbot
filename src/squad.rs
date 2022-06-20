use crate::embed;
use crate::redis_io;
use serenity::model::interactions::application_command::{
    ApplicationCommand, ApplicationCommandInteraction,
    ApplicationCommandInteractionDataOptionValue, ApplicationCommandOptionType,
};
use serenity::model::interactions::InteractionResponseType;
use serenity::model::prelude::Message;
use serenity::model::prelude::message_component::MessageComponentInteraction;
use serenity::prelude::Context;
use serenity::Error;

/// Get squad size argument from /squad command
async fn parse_squad_command(command: &ApplicationCommandInteraction) -> u8 {
    let options = command
        .data
        .options
        .get(0)
        .expect("Expected squad size.")
        .resolved
        .as_ref()
        .expect("Expected integer.");

    if let ApplicationCommandInteractionDataOptionValue::Integer(size) = options {
        u8::try_from(*size).expect("Please provide a valid size.")
    } else {
        panic!("Please provide a valid size.")
    }
}

/// Create initial squad posting
async fn respond_squad_command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    capacity: u8,
) -> Result<Message, Error> {
    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| embed::build_embed(m, capacity))
        })
        .await?;
    command.get_interaction_response(&ctx.http).await
}

/// Globally register /squad command
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

/// Create data for new squad posting
pub async fn handle_squad_command(ctx: &Context, command: &ApplicationCommandInteraction) {
    let capacity: u8 = parse_squad_command(&command).await;
    let command_result = respond_squad_command(&ctx, &command, capacity).await;
    let channel_id = command.channel_id.as_u64().to_string();
    match command_result {
        Ok(response) => {
            let mut con = redis_io::get_redis_connection(&ctx).await;
            let message_id = response.id.as_u64().to_string();
            redis_io::build_squad(&mut con, &channel_id, &message_id, capacity).unwrap();
        }
        Err(_) => {
            println!("Unable to respond to command.");
        }
    }
}

/// Create data for new squad member and update squad posting
pub async fn handle_add_member(
    ctx: &Context,
    interaction: &MessageComponentInteraction,
    expires: u8,
) {
    let message_id = interaction.message.id.as_u64().to_string();
    let user_id = interaction.user.id.as_u64().to_string();
    let seconds: u32 = u32::from(expires) * 60 * 60;
    let mut con = redis_io::get_redis_connection(&ctx).await;
    redis_io::add_member(&mut con, &message_id, &user_id, seconds).unwrap();
    embed::build_message(&ctx, &interaction.channel_id, &mut con, &message_id).await;
}

/// Delete data for interacting user and update squad posting
pub async fn handle_delete_member(ctx: &Context, interaction: &MessageComponentInteraction) {
    let message_id = interaction.message.id.as_u64().to_string();
    let user_id = interaction.user.id.as_u64().to_string();
    let mut con = redis_io::get_redis_connection(&ctx).await;
    redis_io::delete_member(&mut con, &message_id, &user_id).unwrap();
    embed::build_message(&ctx, &interaction.channel_id, &mut con, &message_id).await;
}

/// Determine which button was pressed on the squad posting
pub fn parse_component_id(interaction: &MessageComponentInteraction) -> embed::ButtonChoice {
    let id = interaction.data.custom_id.clone();
    match id.parse() {
        Ok(expires) => embed::ButtonChoice::Hours(expires),
        Err(_) => embed::ButtonChoice::Leave(id),
    }
}

