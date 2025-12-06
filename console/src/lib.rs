use crate::{interface::*, protocol::ConsoleMessage};
use bevy::{ecs::system::SystemId, input_focus::InputFocus, platform::collections::*, prelude::*};

use bevy_ui_text_input::*;
use lightyear::prelude::*;

//mod command;
mod interface;
mod protocol;
mod spawn;

// Minecraft style text chat to enter in commands like "spawn Player" using reflect potentially
pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TextInputPlugin);
        app.init_resource::<ConsoleConfig>();
        app.add_systems(
            Startup,
            (
                interface::spawn_console,
                spawn::add_spawn_command,
                default_commands,
            ),
        );
        app.add_systems(
            Update,
            (registering_systems, submit_text_routing, manage_console),
        );
        app.add_observer(trying_command);
        app.add_observer(output_console_message);
        // This could be a feature crate, everything below here is networking between consoles
        app.register_message::<protocol::ConsoleMessage>()
            .add_direction(NetworkDirection::Bidirectional);
    }
}

/// Note: Spawn is added in the spawn.rs file
fn default_commands(mut console_config: ResMut<ConsoleConfig>) {
    console_config.insert_command("help".to_string(), |_: In<String>| {
        info!("ts don't work rn");
    });
    console_config.insert_command("67".to_string(), |_: In<String>, mut commands: Commands| {
        commands.trigger(ConsoleMessage::new("67".to_string()))
    });
}

#[derive(Resource)]
pub struct ConsoleConfig {
    prefix: char,
    open_close_key: KeyCode,
    // Why not combine the two if they have the same key? idk seems like too much work for insertion
    commands: HashMap<String, (CommandSystem, Option<CommandMetadata>)>,
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            prefix: '/',
            open_close_key: KeyCode::F1,
            commands: HashMap::new(),
        }
    }
}

impl ConsoleConfig {
    pub fn new(prefix: char) -> Self {
        Self {
            prefix,
            ..default()
        }
    }

    pub fn insert_command<M, S: Into<String>>(
        &mut self,
        name: S,
        system: impl IntoSystem<In<String>, (), M> + Send + Sync + 'static,
    ) where
        M: 'static,
    {
        // Store a closure that will register the system when called
        let closure = Box::new(move |mut commands: Commands| commands.register_system(system));
        let name = name.into();
        self.commands
            .insert(name, (CommandSystem::NeedsProcessing(closure), None));
    }

    pub fn insert_command_with_metadata<M, S: Into<String>>(
        &mut self,
        name: S,
        metadata: CommandMetadata,
        system: impl IntoSystem<In<String>, (), M> + Send + Sync + 'static,
    ) where
        M: 'static,
    {
        // Store a closure that will register the system when called
        let closure = Box::new(move |mut commands: Commands| commands.register_system(system));
        let name = name.into();
        self.commands.insert(
            name.clone(),
            (CommandSystem::NeedsProcessing(closure), Some(metadata)),
        );
    }

    pub fn get_commands(&self) -> Vec<&String> {
        self.commands.keys().collect()
    }

    pub fn get_system<S: Into<String>>(&self, name: S) -> Option<&CommandSystem> {
        let name = name.into();
        self.commands.get(&name).map(|(system, _)| system)
    }

    pub fn get_metadata<S: Into<String>>(&self, name: S) -> &Option<CommandMetadata> {
        let name = name.into();
        match self.commands.get(&name) {
            Some((_, metadata)) => metadata,
            None => &None,
        }
    }
}

pub struct CommandMetadata {
    description: String,
    usage: String,
}

// Used to manage command systems
pub enum CommandSystem {
    NeedsProcessing(Box<dyn FnOnce(Commands) -> SystemId<In<String>> + Send + Sync>),
    Processed(SystemId<In<String>>),
}

impl CommandSystem {
    pub fn is_processed(&self) -> bool {
        matches!(self, CommandSystem::Processed(_))
    }

    pub fn get_processed(&self) -> Option<SystemId<In<String>>> {
        match self {
            CommandSystem::Processed(id) => Some(*id),
            _ => None,
        }
    }
}

#[derive(Event, Reflect, Deref, Clone)]
pub struct TryCommand(pub String);

pub fn registering_systems(mut console_config: ResMut<ConsoleConfig>, mut commands: Commands) {
    let mut to_register = Vec::new();

    for (name, (command, _)) in console_config.commands.iter() {
        if !command.is_processed() {
            to_register.push(name.clone());
        }
    }

    for name in to_register {
        if let Some((CommandSystem::NeedsProcessing(register_fn), metadata)) =
            console_config.commands.remove(&name)
        {
            let system_id = register_fn(commands.reborrow());
            console_config
                .commands
                .insert(name, (CommandSystem::Processed(system_id), metadata));
        }
    }
}

pub fn trying_command(
    trigger: On<TryCommand>,
    mut commands: Commands,
    console_config: Res<ConsoleConfig>,
) {
    let try_command = trigger.0.clone();
    let (command_name, arguments) = try_command
        .split_once(' ')
        .unwrap_or((try_command.as_str(), ""));
    if let Some(command) = console_config.get_system(&command_name.to_string()) {
        if let Some(system_id) = command.get_processed() {
            commands.run_system_with(system_id, arguments.to_string());
        }
    }
}

fn submit_text_routing(
    mut messages: MessageReader<SubmitText>,
    mut commands: Commands,
    console_config: Res<ConsoleConfig>,
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
            commands.trigger(TryCommand(command));
        } else {
            commands.trigger(ConsoleMessage::new(message.text.clone()));
        }
    }
}

pub fn manage_console(
    console_config: Res<ConsoleConfig>,
    mut input_focus: ResMut<InputFocus>,
    input: Res<ButtonInput<KeyCode>>,
    mut visibility: Single<&mut Visibility, With<Console>>,
    console_command_line: Single<(Entity, &mut TextInputNode), With<ConsoleCommandLine>>,
) {
    if input.just_pressed(console_config.open_close_key) {
        let (entity, mut text_input_node) = console_command_line.into_inner();
        text_input_node.is_enabled = !text_input_node.is_enabled;
        visibility.toggle_visible_hidden();
        if text_input_node.is_enabled {
            input_focus.set(entity);
        } else if input_focus.0.is_some() && input_focus.0.unwrap() == entity {
            input_focus.clear();
        }
    }
}

pub fn output_console_message(
    console_message: On<ConsoleMessage>,
    mut commands: Commands,
    console_message_container: Single<Entity, With<ConsoleMessageContainer>>,
) {
    let output = commands
        .spawn((
            console_output(console_message.message.clone()),
            TextColor(console_message.color),
        ))
        .id();
    commands
        .entity(console_message_container.entity())
        .add_child(output);
}
