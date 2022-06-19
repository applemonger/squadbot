use crate::embed;
use crate::redis_core;
use serenity::model::prelude::message_component::MessageComponentInteraction;
use serenity::prelude::Context;

pub async fn handle_add_member(
    ctx: &Context,
    interaction: &MessageComponentInteraction,
    expires: u8,
) {
    let message_id = interaction.message.id.as_u64().to_string();
    let user_id = interaction.user.id.as_u64().to_string();
    let seconds: u32 = u32::from(expires) * 60 * 60;
    let mut con = redis_core::get_redis_connection(&ctx).await;
    redis_core::add_member(&mut con, &message_id, &user_id, seconds).unwrap();
    embed::build_message(&ctx, &interaction.channel_id, &mut con, &message_id).await;
}

pub async fn handle_delete_member(ctx: &Context, interaction: &MessageComponentInteraction) {
    let message_id = interaction.message.id.as_u64().to_string();
    let user_id = interaction.user.id.as_u64().to_string();
    let mut con = redis_core::get_redis_connection(&ctx).await;
    redis_core::delete_member(&mut con, &message_id, &user_id).unwrap();
    embed::build_message(&ctx, &interaction.channel_id, &mut con, &message_id).await;
}

pub fn parse_component_id(interaction: &MessageComponentInteraction) -> embed::ButtonChoice {
    let id = interaction.data.custom_id.clone();
    match id.parse() {
        Ok(expires) => embed::ButtonChoice::Hours(expires),
        Err(_) => embed::ButtonChoice::Leave(id),
    }
}
