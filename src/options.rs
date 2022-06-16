use std::collections::HashMap;
use serenity::model::prelude::ReactionType;
use std::str::FromStr;

lazy_static! {
    pub static ref OPTIONS = HashMap::from([
        (ReactionType::from_str("1️⃣").unwrap(), 1),
        (ReactionType::from_str("2️⃣").unwrap(), 2),
        (ReactionType::from_str("3️⃣").unwrap(), 3),
        (ReactionType::from_str("4️⃣").unwrap(), 4),
        (ReactionType::from_str("5️⃣").unwrap(), 5),
        (ReactionType::from_str("6️⃣").unwrap(), 6),
        (ReactionType::from_str("7️⃣").unwrap(), 7),
        (ReactionType::from_str("8️⃣").unwrap(), 8),
        (ReactionType::from_str("9️⃣").unwrap(), 9),
    ])
}