use serenity::builder::CreateInteractionResponseData;
use serenity::builder::{CreateActionRow, CreateButton};
use serenity::model::interactions::message_component::ButtonStyle;
use serenity::utils::Colour;

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

fn action_row() -> CreateActionRow {
    let mut ar = CreateActionRow::default();
    ar.add_button(button(ButtonChoice::Hours(1)));
    ar.add_button(button(ButtonChoice::Hours(2)));
    ar.add_button(button(ButtonChoice::Hours(3)));
    ar.add_button(button(ButtonChoice::Hours(4)));
    ar.add_button(button(ButtonChoice::Other(String::from("Leave"))));
    ar
}

fn create_description(content: &String) -> String {
    format!(
        "✅ React to this message to ready up!\n\
        1️⃣ Use the number reacts to indicate for how many hours you are available.\n\n\
        SquadBot will message you when at least {} people are ready.\n\n",
        content
    )
}

pub fn build_embed<'a, 'b>(
    m: &'b mut CreateInteractionResponseData<'a>,
    content: &String,
) -> &'b mut CreateInteractionResponseData<'a> {
    m.embed(|e| {
        e.title("Assemble your squad!");
        e.description(create_description(&content));
        e.colour(Colour::from_rgb(59, 165, 93));
        return e;
    });
    m.components(|c| c.add_action_row(action_row()));
    m
}
