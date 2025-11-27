use bevy::prelude::*;
use serde::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConsoleMessage(pub String);
