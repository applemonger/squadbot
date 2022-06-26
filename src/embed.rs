use redis;
use crate::redis_io;
use crate::redis_io::SquadStatus;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateComponents, CreateInteractionResponseData, EditMessage,
};
use serenity::client::Context;
use serenity::model::id::{ChannelId, MessageId, UserId};
use serenity::model::interactions::message_component::ButtonStyle;
use serenity::model::mention::Mention;
use serenity::utils::Colour;
use std::collections::HashMap;
use std::error::Error;

pub enum ButtonChoice {
    Hours(u8),
    Leave(String),
}

/// Creates a message component button, which can either be an hour selection or a
/// "Leave Squad" button.
fn button(choice: ButtonChoice) -> CreateButton {
    let mut b = CreateButton::default();
    match choice {
        ButtonChoice::Hours(hours) => {
            b.custom_id(hours.to_string().to_ascii_lowercase());
            b.label(hours.to_string());
            b.style(ButtonStyle::Primary);
        }
        ButtonChoice::Leave(s) => {
            b.custom_id(&s);
            b.label(&s);
            b.style(ButtonStyle::Danger);
        }
    }
    b
}

/// Build first row of message component buttons (Maximum 5).
fn hours_selection_row_1() -> CreateActionRow {
    let mut ar = CreateActionRow::default();
    ar.add_button(button(ButtonChoice::Hours(1)));
    ar.add_button(button(ButtonChoice::Hours(2)));
    ar.add_button(button(ButtonChoice::Hours(3)));
    ar.add_button(button(ButtonChoice::Hours(4)));
    ar.add_button(button(ButtonChoice::Hours(5)));
    ar
}

/// Build second row of message component buttons (Maximum 5).
fn hours_selection_row_2() -> CreateActionRow {
    let mut ar = CreateActionRow::default();
    ar.add_button(button(ButtonChoice::Hours(6)));
    ar.add_button(button(ButtonChoice::Hours(7)));
    ar.add_button(button(ButtonChoice::Hours(8)));
    ar.add_button(button(ButtonChoice::Hours(9)));
    ar.add_button(button(ButtonChoice::Hours(10)));
    ar
}

/// Build last row of message component buttons.
fn options_row() -> CreateActionRow {
    let mut ar = CreateActionRow::default();
    ar.add_button(button(ButtonChoice::Leave(String::from("Leave Squad"))));
    ar
}

/// Assemble all rows of action buttons into one component.
fn action_rows(c: &mut CreateComponents) -> &mut CreateComponents {
    c.add_action_row(hours_selection_row_1());
    c.add_action_row(hours_selection_row_2());
    c.add_action_row(options_row());
    c
}

/// Color of side of embed
fn get_colour() -> Colour {
    Colour::from_rgb(59, 165, 93)
}

/// Base description included on forming squad postings.
pub fn create_description(capacity: u8) -> String {
    format!(
        "1️⃣ Use the number reacts to indicate for how many hours you are available.\n\n\
        SquadBot will message you when at least {} people are ready.\n\n",
        capacity.to_string()
    )
}

/// Formats seconds to readable time syntax.
pub fn format_ttl(ttl: u64) -> String {
    let minutes = ttl / 60;
    let hours = minutes / 60;
    let minutes = minutes % 60;
    match hours {
        0 => format!("{}m", minutes),
        _ => format!("{}h {}m", hours, minutes),
    }
}

/// Used to build the initial squad posting
pub fn build_embed<'a, 'b>(
    m: &'b mut CreateInteractionResponseData<'a>,
    capacity: u8,
) -> &'b mut CreateInteractionResponseData<'a> {
    let description = create_description(capacity);
    m.embed(|e| {
        e.title("Assemble your squad!");
        e.description(description);
        e.colour(get_colour());
        e
    });
    m.components(|c| action_rows(c));
    m
}

/// Build embed description dependent upon squad status
/// Forming squad: Displays current squad members, their availability, and remaining
///     duration of the squad posting
/// Filled squad: Displays the filled squad roster.
/// Expired squad: Mostly blank embed.
pub fn build_description(
    con: &mut redis::Connection,
    squad_id: &String,
    squad_status: &SquadStatus,
    message_id: &String,
) -> Result<String, redis::RedisError> {
    // Build description based on squad status.
    let description = match squad_status {
        SquadStatus::Expired => String::from("🔴 This squad has expired."),
        SquadStatus::Forming => {
            let capacity: u8 = redis_io::get_capacity(con, &squad_id)?;
            let members: HashMap<UserId, u64> = redis_io::get_members(con, &squad_id)?;
            let posting_id = redis_io::posting_id(&message_id);
            let posting_ttl = redis_io::get_ttl(con, &posting_id)?;
            let base_description = create_description(capacity);
            let mut roster = String::new();
            for (key, value) in &members {
                let mention = format!("{}", Mention::from(*key));
                let ttl = format_ttl(*value);
                let line = &format!("{} available for {}\n", mention, ttl)[..];
                roster.push_str(line);
            }
            let status = format!(
                "🟡 This squad is still forming. Time left: {}",
                format_ttl(posting_ttl)
            );
            format!(
                "{}**Current Squad**\n{}\n{}",
                base_description, roster, status
            )
        }
        SquadStatus::Filled => {
            let mut roster = String::new();
            let members: HashMap<UserId, u64> = redis_io::get_members(con, &squad_id)?;
            for (key, _value) in &members {
                let mention = format!("{}", Mention::from(*key));
                let line = &format!("{}\n", mention)[..];
                roster.push_str(line);
            }
            format!(
                "**Squad**\n{}\n{}",
                roster,
                String::from("🟢 This squad has been filled!")
            )
        }
    };
    
    Ok(description)
}

/// Used to update the posting after it has been created.
/// Forming squad: Displays buttons to join and leave squad.
/// Filled squad: Buttons are removed.
/// Expired squad: Buttons are removed.
pub fn update_embed<'a, 'b>(
    m: &'b mut EditMessage<'a>,
    squad_status: SquadStatus,
    description: &String,
) -> &'b mut EditMessage<'a> {
    // Build embed
    m.embed(|e| {
        e.title("Assemble your squad!");
        e.description(description);
        e.colour(get_colour());
        e
    });

    // Add or remove interaction buttons based on squad status.
    match squad_status {
        SquadStatus::Forming => {
            m.components(|c| action_rows(c));
        }
        _ => {
            m.set_components(CreateComponents(Vec::new()));
        }
    };
    m
}

/// Sends updated squad posting to channel.
pub async fn build_message(
    ctx: &Context,
    channel_id: &ChannelId,
    con: &mut redis::Connection,
    message_id: &String,
) -> Result<(), Box<dyn Error>> {
    let squad_id = redis_io::squad_id(&message_id);
    let squad_status = redis_io::get_squad_status(con, &squad_id)?;
    let description = build_description(con, &squad_id, &squad_status, &message_id)?;
    let message_id_u64 = message_id.parse()?;
    channel_id
        .edit_message(&ctx, MessageId(message_id_u64), |m| {
            update_embed(m, squad_status, &description)
        })
        .await?;
    Ok(())
}
