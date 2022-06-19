use crate::embed;
use crate::redis_core;
use serenity::model::id::{ChannelId, MessageId};
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
    build_message(&ctx, &interaction.channel_id, &mut con, &message_id).await;
}

pub async fn handle_delete_member(ctx: &Context, interaction: &MessageComponentInteraction) {
    let message_id = interaction.message.id.as_u64().to_string();
    let user_id = interaction.user.id.as_u64().to_string();
    let mut con = redis_core::get_redis_connection(&ctx).await;
    redis_core::delete_member(&mut con, &message_id, &user_id).unwrap();
    build_message(&ctx, &interaction.channel_id, &mut con, &message_id).await;
}

pub async fn build_message(
    ctx: &Context,
    channel_id: &ChannelId,
    con: &mut redis::Connection,
    message_id: &String,
) {
    let capacity: u8 = redis_core::get_capacity(con, &message_id).unwrap();
    let description =
        embed::create_description_with_members(con, &capacity.to_string(), &message_id);
    channel_id
        .edit_message(&ctx, MessageId(message_id.parse().unwrap()), |m| {
            embed::update_embed(m, description)
        })
        .await
        .unwrap();
}

pub fn parse_component_id(interaction: &MessageComponentInteraction) -> embed::ButtonChoice {
    let id = interaction.data.custom_id.clone();
    match id.parse() {
        Ok(expires) => embed::ButtonChoice::Hours(expires),
        Err(_) => embed::ButtonChoice::Other(id),
    }
}
