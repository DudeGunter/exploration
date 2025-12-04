use bevy::{platform::collections::*, prelude::*};

#[derive(PartialEq, Eq, Hash)]
pub struct Command(String);

pub trait Functionality<const T: u64> {}

#[derive(Event)]
pub struct Call<const T: u64>;

#[derive(Resource)]
pub struct ConsoleConfig {
    perfix: char,
    commands: HashSet<Command>,
}

impl ConsoleConfig {
    pub fn insert_command(&mut self, command: Command) {
        self.commands.insert(command);
    }

    pub fn get_commands(&self) -> Vec<&Command> {
        self.commands.iter().collect()
    }
}
