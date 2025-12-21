use bevy::prelude::*;
use serde::*;

/// ConsoleMessage wrapper with formatting
#[macro_export]
macro_rules! message {
    ($($arg:tt)*) => {
        $crate::ConsoleMessage::new(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! error_message {
    ($($arg:tt)*) => {
        $crate::ConsoleMessage::error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! warning_message {
    ($($arg:tt)*) => {
        $crate::ConsoleMessage::warning(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! success_message {
    ($($arg:tt)*) => {
        $crate::ConsoleMessage::success(&format!($($arg)*))
    };
}

/// ConsoleMessage wrapper without formatting
pub fn message<S: Into<String>>(message: S) -> ConsoleMessage {
    ConsoleMessage::new(message.into())
}

///BIG NOTE: it could be more effiecent if a large amount of lines are being outputed
/// to send them as a vec or list of somesort as to not run the same observer 100x times over
#[derive(Component, Event, Serialize, Deserialize, Clone, Debug, PartialEq)]
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

    pub fn with_color(&mut self, color: Color) -> Self {
        self.color = color;
        self.clone()
    }
    // Why use new? in the case of new features, you can rely on new doing most of the heavy lifting
    pub fn error<S: Into<String>>(message: S) -> Self {
        Self::new(message).with_color(Color::srgb(1.0, 0.4, 0.4))
    }

    pub fn success<S: Into<String>>(message: S) -> Self {
        Self::new(message).with_color(Color::srgb(0.4, 1.0, 0.4))
    }

    pub fn warning<S: Into<String>>(message: S) -> Self {
        Self::new(message).with_color(Color::srgb(1.0, 1.0, 0.4))
    }
}
