use bevy::prelude::*;
use bevy_ui_text_input::*;

// Minecraft style text chat to enter in commands like "spawn Player" using reflect potentially
pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TextInputPlugin);
        app.add_systems(Startup, spawn_console);
        app.add_systems(Update, print_line_to_console);
        app.add_observer(handle_command);
    }
}

#[derive(Component, Debug, Reflect)]
pub struct Console;

#[derive(Component, Debug, Reflect)]
pub struct ConsoleMessageContainer;

pub fn spawn_console(mut cmds: Commands) {
    cmds.spawn((
        Name::new("Console"),
        Console,
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::ColumnReverse,
            ..default()
        },
        children![
            (
                Name::new("Command Line"),
                Node {
                    width: percent(100),
                    height: px(24),
                    ..default()
                },
                TextInputNode {
                    mode: TextInputMode::SingleLine,
                    clear_on_submit: true,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ),
            (
                Name::new("Console Message Container"),
                ConsoleMessageContainer,
                Node {
                    width: percent(100),
                    align_self: AlignSelf::FlexEnd,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            )
        ],
    ));
}

// This might not work if the bevy_ui_text_input is used anywhere else, it would pick up the message
fn print_line_to_console(
    mut messages: MessageReader<SubmitText>,
    mut cmds: Commands,
    console_message_container: Single<Entity, With<ConsoleMessageContainer>>,
) {
    for message in messages.read() {
        if message.text.is_empty() {
            continue;
        }
        if message.text.starts_with('/') {
            let mut command = message.text.clone();
            command.remove(0);
            cmds.trigger(TryCommand(command));
        } else {
            let output = cmds.spawn(command_line_output(message.text.clone())).id();
            cmds.entity(console_message_container.entity())
                .add_child(output);
        }
    }
}

fn command_line_output(text: String) -> impl Bundle {
    (
        Name::new(format!("Message: {text}")),
        Node {
            width: percent(100),
            height: px(35),
            ..default()
        },
        Text::new(text),
    )
}

#[derive(Event, Reflect, Deref, Clone)]
pub struct TryCommand(pub String);

fn handle_command(
    trigger: On<TryCommand>,
    mut commands: Commands,
    console_message_container: Single<Entity, With<ConsoleMessageContainer>>,
) {
    let (cmd, args) = trigger
        .event()
        .split_once(' ')
        .unwrap_or((trigger.event(), ""));
    let mut outputs: Vec<Entity> = Vec::new();
    let mut out = |message: &str| {
        outputs.push(
            commands
                .spawn(command_line_output(message.to_string()))
                .id(),
        );
    };
    match cmd {
        "help" => out("Help me"),
        "spawn" => {
            out("Spawning");
            out(args);
        }
        _ => {}
    }
    commands
        .entity(console_message_container.entity())
        .add_children(outputs.as_slice());
}
