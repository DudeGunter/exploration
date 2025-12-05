use crate::{command::*, *};
use bevy::{ecs::system::SystemId, platform::collections::*, prelude::*};

#[derive(Resource)]
pub struct ConsoleConfig {
    prefix: char,
    commands: HashMap<String, Command>,
}

enum Command {
    NeedsProcessing(Box<dyn FnOnce(&mut World) -> SystemId + Send + Sync>),
    Processed(SystemId),
}

impl Command {
    pub fn is_processed(&self) -> bool {
        matches!(self, Command::Processed(_))
    }

    pub fn get_processed(&self) -> Option<SystemId> {
        match self {
            Command::Processed(id) => Some(*id),
            _ => None,
        }
    }
}

impl ConsoleConfig {
    pub fn new(prefix: char) -> Self {
        Self {
            prefix,
            commands: HashMap::new(),
        }
    }

    /// WARNING: This requires heavy World access, it shouldn't be called often or during the main game loop
    pub fn insert_command<M>(
        &mut self,
        name: String,
        system: impl IntoSystem<(), (), M> + Send + Sync + 'static,
    ) where
        M: 'static,
    {
        // Store a closure that will register the system when called
        let closure = Box::new(move |world: &mut World| world.register_system(system));
        self.commands
            .insert(name, Command::NeedsProcessing(closure));
    }

    pub fn get_commands(&self) -> Vec<&String> {
        self.commands.keys().collect()
    }
}

pub fn handle_registering_systems(world: &mut World) {
    let mut to_register = Vec::new();

    // Collect names of systems that need registration
    if let Some(console_config) = world.get_resource::<ConsoleConfig>() {
        for (name, command) in console_config.commands.iter() {
            if matches!(command, Command::NeedsProcessing(_)) {
                to_register.push(name.clone());
            }
        }
    }

    // Register each system
    for name in to_register {
        let system_id = if let Some(mut console_config) = world.get_resource_mut::<ConsoleConfig>()
        {
            if let Some(Command::NeedsProcessing(register_fn)) =
                console_config.commands.remove(&name)
            {
                Some(register_fn(world))
            } else {
                None
            }
        } else {
            None
        };

        // Store the SystemId back
        if let (Some(system_id), Some(mut console_config)) =
            (system_id, world.get_resource_mut::<ConsoleConfig>())
        {
            console_config
                .commands
                .insert(name, Command::Processed(system_id));
        }
    }
}

pub fn handle_trying_command(
    trigger: On<TryCommand>,
    mut commands: Commands,
    console_config: Res<ConsoleConfig>,
) {
    let command_name = trigger.0.clone();
    if let Some(command) = console_config.commands.get(&command_name) {
        if let Some(system_id) = command.get_processed() {
            commands.run_system(system_id);
        }
    }
}

fn handle_submit_text_routing(
    mut messages: MessageReader<SubmitText>,
    mut commands: Commands,
    console_config: Res<ConsoleConfig>,
    console_message_container: Single<Entity, With<ConsoleMessageContainer>>,
    console_command_line: Single<Entity, With<ConsoleCommandLine>>,
) {
    let console_entity = console_command_line.into_inner();
    for message in messages.read() {
        if message.text.is_empty() && message.entity == console_entity {
            continue;
        }
        if message.text.starts_with(console_config.prefix) {
            let mut command = message.text.clone();
            command.remove(0);
            commands.trigger(command::TryCommand(command));
        } else {
            let output = commands
                .spawn(command_line_output(message.text.clone()))
                .id();
            commands
                .entity(console_message_container.entity())
                .add_child(output);
        }
    }
}
