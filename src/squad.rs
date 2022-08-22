use crate::embed;
use crate::redis_io;
use rand::Rng;
use serenity::model::id::RoleId;
use serenity::model::interactions::application_command::{
    ApplicationCommand, ApplicationCommandInteraction, ApplicationCommandInteractionDataOption,
    ApplicationCommandInteractionDataOptionValue, ApplicationCommandOptionType,
};
use serenity::model::interactions::InteractionResponseType;
use serenity::model::prelude::message_component::MessageComponentInteraction;
use serenity::model::prelude::Message;
use serenity::prelude::Context;
use serenity::Error;
use serenity::prelude::Mentionable;
use std::error::Error as StdError;

/// Get squad size argument from /squad command
async fn parse_squad_size(
    command: &ApplicationCommandInteraction,
) -> Result<Option<u8>, Box<dyn StdError>> {
    let options: Vec<&ApplicationCommandInteractionDataOption> = command
        .data
        .options
        .iter()
        .filter(|opt| opt.name == "size")
        .collect();

    let option = options.get(0);

    let option = match option {
        Some(opt) => opt,
        None => {
            return Ok(Some(5));
        }
    };

    let option = option.resolved.as_ref();

    let option = match option {
        Some(opt) => opt,
        None => {
            return Ok(Some(5));
        }
    };

    let size = match option {
        ApplicationCommandInteractionDataOptionValue::Integer(size) => size,
        _ => {
            return Err("Unable to parse size.".into());
        }
    };

    let size = u8::try_from(*size)?;
    Ok(Some(size))
}

/// Get squad role argument from /squad command
async fn parse_squad_role(
    command: &ApplicationCommandInteraction,
) -> Result<Option<RoleId>, Box<dyn StdError>> {
    let options: Vec<&ApplicationCommandInteractionDataOption> = command
        .data
        .options
        .iter()
        .filter(|opt| opt.name == "role")
        .collect();

    let option = options.get(0);

    let option = match option {
        Some(opt) => opt,
        None => {
            return Ok(None);
        }
    };

    let option = option.resolved.as_ref();

    let option = match option {
        Some(opt) => opt,
        None => {
            return Ok(None);
        }
    };

    let role = match option {
        ApplicationCommandInteractionDataOptionValue::Role(role) => role,
        _ => {
            return Err("Unable to parse role.".into());
        }
    };

    Ok(Some(role.id))
}

/// Get squad role argument from /squad command
async fn parse_squad_id(
    command: &ApplicationCommandInteraction,
) -> Result<Option<String>, Box<dyn StdError>> {
    let options: Vec<&ApplicationCommandInteractionDataOption> = command
        .data
        .options
        .iter()
        .filter(|opt| opt.name == "id")
        .collect();

    let option = options.get(0);

    let option = match option {
        Some(opt) => opt,
        None => {
            return Ok(None);
        }
    };

    let option = option.resolved.as_ref();

    let option = match option {
        Some(opt) => opt,
        None => {
            return Ok(None);
        }
    };

    let id: String = match option {
        ApplicationCommandInteractionDataOptionValue::String(id) => id.to_string(),
        _ => {
            return Err("Unable to parse id.".into());
        }
    };

    Ok(Some(id))
}

/// Create initial squad posting
async fn respond_squad_command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    squad_id: &String,
    capacity: u8,
    role_id: Option<RoleId>,
) -> Result<Message, Error> {
    if let Some(role) = role_id {
        command.channel_id.say(&ctx.http, format!("Squad forming! {}", role.mention())).await?;
    }
    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| embed::build_embed(m, &squad_id, capacity, role_id))
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
                    .description("Number from 1 to 10")
                    .kind(ApplicationCommandOptionType::Integer)
                    .min_int_value(1)
                    .max_int_value(10)
                    .required(false)
            })
            .create_option(|option| {
                option
                    .name("role")
                    .description("Tag a role e.g. @gamers, @valorant, etc.")
                    .kind(ApplicationCommandOptionType::Role)
                    .required(false)
            })
            .create_option(|option| {
                option
                    .name("id")
                    .description("ID of another posting for cross-server squads.")
                    .kind(ApplicationCommandOptionType::String)
                    .required(false)
            })
    })
    .await
}

/// Create data for new squad posting
pub async fn handle_squad_command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), Box<dyn StdError>> {
    let capacity: Option<u8> = parse_squad_size(&command).await?;
    let role_id: Option<RoleId> = parse_squad_role(&command).await?;
    let squad_id: Option<String> = parse_squad_id(&command).await?;
    let mut con = redis_io::get_redis_connection(&ctx).await?;
    match squad_id {
        Some(id) => {
            let capacity = redis_io::get_capacity(&mut con, &id)?;
            let response = respond_squad_command(&ctx, &command, &id, capacity, role_id).await?;
            let channel_id = command.channel_id.as_u64().to_string();
            let message_id = response.id.as_u64().to_string();
            redis_io::build_posting(&mut con, &channel_id, &message_id, role_id, &id)?;
        }
        None => {
            let id = generate_squad_id();
            let capacity = match capacity {
                Some(n) => n,
                None => 5,
            };
            redis_io::build_squad(&mut con, &id, capacity)?;
            let response = respond_squad_command(&ctx, &command, &id, capacity, role_id).await?;
            let channel_id = command.channel_id.as_u64().to_string();
            let message_id = response.id.as_u64().to_string();
            redis_io::build_posting(&mut con, &channel_id, &message_id, role_id, &id)?;
        }
    }

    Ok(())
}

/// Create data for new squad member and update squad posting
pub async fn handle_add_member(
    ctx: &Context,
    interaction: &MessageComponentInteraction,
    expires: u8,
) -> Result<(), Box<dyn StdError>> {
    let message_id = interaction.message.id.as_u64().to_string();
    let user_id = interaction.user.id.as_u64().to_string();
    let seconds: u32 = u32::from(expires) * 60 * 60;
    let mut con = redis_io::get_redis_connection(&ctx).await?;
    let squad_id = redis_io::get_squad_id(&mut con, &message_id)?;
    redis_io::add_member(&mut con, &squad_id, &user_id, seconds)?;
    embed::build_message(&ctx, &interaction.channel_id, &mut con, &message_id).await?;
    Ok(())
}

/// Delete data for interacting user and update squad posting
pub async fn handle_delete_member(
    ctx: &Context,
    interaction: &MessageComponentInteraction,
) -> Result<(), Box<dyn StdError>> {
    let message_id = interaction.message.id.as_u64().to_string();
    let user_id = interaction.user.id.as_u64().to_string();
    let mut con = redis_io::get_redis_connection(&ctx).await?;
    let squad_id = redis_io::get_squad_id(&mut con, &message_id)?;
    redis_io::delete_member(&mut con, &squad_id, &user_id)?;
    embed::build_message(&ctx, &interaction.channel_id, &mut con, &message_id).await?;
    Ok(())
}

/// Determine which button was pressed on the squad posting
pub fn parse_component_id(interaction: &MessageComponentInteraction) -> embed::ButtonChoice {
    let id = interaction.data.custom_id.clone();
    match id.parse() {
        Ok(expires) => embed::ButtonChoice::Hours(expires),
        Err(_) => embed::ButtonChoice::Leave(id),
    }
}

/// Generate a random squad id
pub fn generate_squad_id() -> String {
    let mut rng = rand::thread_rng();
    let rand_id: u32 = rng.gen();
    format!("squad:{}", rand_id.to_string())
}
