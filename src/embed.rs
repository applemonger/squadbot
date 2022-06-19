use crate::redis_core;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateComponents, CreateInteractionResponseData, EditMessage,
};
use serenity::client::Context;
use serenity::model::id::{ChannelId, MessageId, UserId};
use serenity::model::interactions::message_component::ButtonStyle;
use serenity::model::mention::Mention;
use serenity::utils::Colour;
use std::collections::HashMap;

pub enum ButtonChoice {
    Hours(u8),
    Other(String),
}

fn button(choice: ButtonChoice) -> CreateButton {
    let mut b = CreateButton::default();
    match choice {
        ButtonChoice::Hours(hours) => {
            b.custom_id(hours.to_string().to_ascii_lowercase());
            b.label(hours.to_string());
            b.style(ButtonStyle::Primary);
        }
        ButtonChoice::Other(s) => {
            b.custom_id(&s);
            b.label(&s);
            b.style(ButtonStyle::Danger);
        }
    }
    b
}

fn hours_selection_row_1() -> CreateActionRow {
    let mut ar = CreateActionRow::default();
    ar.add_button(button(ButtonChoice::Hours(1)));
    ar.add_button(button(ButtonChoice::Hours(2)));
    ar.add_button(button(ButtonChoice::Hours(3)));
    ar.add_button(button(ButtonChoice::Hours(4)));
    ar.add_button(button(ButtonChoice::Hours(5)));
    ar
}

fn hours_selection_row_2() -> CreateActionRow {
    let mut ar = CreateActionRow::default();
    ar.add_button(button(ButtonChoice::Hours(6)));
    ar.add_button(button(ButtonChoice::Hours(7)));
    ar.add_button(button(ButtonChoice::Hours(8)));
    ar.add_button(button(ButtonChoice::Hours(9)));
    ar.add_button(button(ButtonChoice::Hours(10)));
    ar
}

fn options_row() -> CreateActionRow {
    let mut ar = CreateActionRow::default();
    ar.add_button(button(ButtonChoice::Other(String::from("Leave Squad"))));
    ar
}

fn action_rows(c: &mut CreateComponents) -> &mut CreateComponents {
    c.add_action_row(hours_selection_row_1());
    c.add_action_row(hours_selection_row_2());
    c.add_action_row(options_row());
    c
}

fn get_colour() -> Colour {
    Colour::from_rgb(59, 165, 93)
}

pub fn create_description(capacity: &String) -> String {
    format!(
        "✅ React to this message to ready up!\n\
        1️⃣ Use the number reacts to indicate for how many hours you are available.\n\n\
        SquadBot will message you when at least {} people are ready.\n\n",
        capacity
    )
}

pub fn create_description_with_members(
    con: &mut redis::Connection,
    capacity: &String,
    message_id: &String,
) -> String {
    let members: HashMap<UserId, u64> = redis_core::get_members(con, &message_id).unwrap();
    let squad_id = redis_core::squad_id(&message_id);
    let status = redis_core::get_squad_status(con, &squad_id).unwrap();
    match status {
        0 => {
            String::from("🔴 This squad has expired.")
        },
        1 => {
            let base_description = create_description(&capacity);
            let mut roster = String::new();
            for (key, value) in &members {
                let mention = format!("{}", Mention::from(*key));
                let ttl = format_ttl(*value);
                let line = &format!("{} available for {}\n", mention, ttl)[..];
                roster.push_str(line);
            }
            format!(
                "{}**Current Squad**\n{}\n{}",
                base_description, roster, String::from("🟡 This squad is still forming.")
            )
        },
        2 => {
            let mut roster = String::new();
            for (key, _value) in &members {
                let mention = format!("{}", Mention::from(*key));
                let line = &format!("{}\n", mention)[..];
                roster.push_str(line);
            }
            format!(
                "**Squad**\n{}\n{}",
                roster, String::from("🟢 This squad has been filled!")
            )
        },
        _ => {
            panic!("Unexpected squad status.")
        }
    }
}

pub fn format_ttl(ttl: u64) -> String {
    let minutes = ttl / 60;
    let hours = minutes / 60;
    let minutes = minutes % 60;
    match hours {
        0 => format!("{}m", minutes),
        _ => format!("{}h {}m", hours, minutes),
    }
}

pub fn build_embed<'a, 'b>(
    m: &'b mut CreateInteractionResponseData<'a>,
    description: String,
) -> &'b mut CreateInteractionResponseData<'a> {
    m.embed(|e| {
        e.title("Assemble your squad!");
        e.description(description);
        e.colour(get_colour());
        e
    });
    m.components(|c| action_rows(c));
    m
}

pub fn update_embed<'a, 'b>(
    m: &'b mut EditMessage<'a>,
    description: String,
    squad_status: u8,
) -> &'b mut EditMessage<'a> {
    m.embed(|e| {
        e.title("Assemble your squad!");
        e.description(description);
        e.colour(get_colour());
        e
    });
    if squad_status == 1 {
        m.components(|c| action_rows(c));
    } else {
        m.set_components(CreateComponents(Vec::new()));
    }
    m
}

pub async fn build_message(
    ctx: &Context,
    channel_id: &ChannelId,
    con: &mut redis::Connection,
    message_id: &String,
) {
    let capacity: u8 = redis_core::get_capacity(con, &message_id).unwrap();
    let description = create_description_with_members(con, &capacity.to_string(), &message_id);
    let squad_id = redis_core::squad_id(&message_id);
    let squad_status = redis_core::get_squad_status(con, &squad_id).unwrap();
    channel_id
        .edit_message(&ctx, MessageId(message_id.parse().unwrap()), |m| {
            update_embed(m, description, squad_status)
        })
        .await
        .unwrap();
}
