use serenity::prelude::Context;
use serenity::model::prelude::message_component::MessageComponentInteraction;
use crate::redis_core;
use crate::embed;

pub async fn handle_add_member(ctx: &Context, interaction: &MessageComponentInteraction, expires: u8) {
    let message_id = interaction.message.id.as_u64().to_string();
    let user_id = interaction.user.id.as_u64().to_string();
    let seconds: u32 = u32::from(expires) * 60 * 60;
    let mut con = redis_core::get_redis_connection(&ctx).await;
    redis_core::add_member(&mut con, &message_id, &user_id, seconds).unwrap();
}

pub fn parse_component_id(
    interaction: &MessageComponentInteraction,
) -> embed::ButtonChoice {
    let id = interaction.data.custom_id.clone();
    match id.parse() {
        Ok(expires) => embed::ButtonChoice::Hours(expires),
        Err(_) => embed::ButtonChoice::Other(id),
    }
}