use bevy::prelude::*;
use serde::*;

///BIG NOTE: it could be more effiecent if a large amount of lines are being outputed
/// to send them as a vec or list of somesort as to not run the same observer 100x times over
#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConsoleMessage {
    pub message: String,
    pub color: Color,
}

// this is here for that ..default notation for later on when there are more fields
impl Default for ConsoleMessage {
    fn default() -> Self {
        ConsoleMessage {
            message: "Uh Oh! You didn't configure a console message!".to_string(),
            color: Color::WHITE,
        }
    }
}

impl ConsoleMessage {
    pub fn new<S: Into<String>>(message: S) -> Self {
        ConsoleMessage {
            message: message.into(),
            ..default()
        }
    }
}
