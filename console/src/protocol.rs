use bevy::prelude::*;
use serde::*;

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConsoleMessage {
    pub message: String,
    pub color: Color,
}

impl Default for ConsoleMessage {
    fn default() -> Self {
        ConsoleMessage {
            message: "Uh Oh! You didn't configure a console message!".to_string(),
            color: Color::WHITE,
        }
    }
}

impl ConsoleMessage {
    pub fn new(message: String) -> Self {
        ConsoleMessage {
            message,
            ..default()
        }
    }
}
