use bevy::prelude::*;
use bevy_ui_text_input::*;

mod command;

// Minecraft style text chat to enter in commands like "spawn Player" using reflect potentially
pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TextInputPlugin);
        app.add_systems(Startup, spawn_console);
        app.add_systems(
            Update,
            (
                print_line_to_console,
                command::spawn_reflected,
                manage_console,
            ),
        );
        app.add_observer(command::handle_command);

        app.add_message::<command::SpawnReflected>();
    }
}

#[derive(Component, Debug, Reflect, Default)]
#[reflect(Component, Default)]
pub struct Console;

#[derive(Component, Debug, Reflect, Default)]
pub struct ConsoleCommandLine;

#[derive(Component, Debug, Reflect)]
pub struct ConsoleMessageContainer;

pub fn spawn_console(mut cmds: Commands) {
    cmds.spawn((
        Name::new("Console"),
        Console,
        Visibility::Hidden,
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::ColumnReverse,
            ..default()
        },
        children![
            (
                Name::new("Command Line"),
                ConsoleCommandLine,
                Node {
                    width: percent(100),
                    height: px(24),
                    ..default()
                },
                TextInputNode {
                    mode: TextInputMode::SingleLine,
                    clear_on_submit: true,
                    is_enabled: false,
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
            cmds.trigger(command::TryCommand(command));
        } else {
            let output = cmds.spawn(command_line_output(message.text.clone())).id();
            cmds.entity(console_message_container.entity())
                .add_child(output);
        }
    }
}

pub fn manage_console(
    input: Res<ButtonInput<KeyCode>>,
    mut visibility: Single<&mut Visibility, With<Console>>,
    mut console_command_line: Single<&mut TextInputNode, With<ConsoleCommandLine>>,
) {
    if input.just_pressed(KeyCode::Tab) {
        console_command_line.is_enabled = !console_command_line.is_enabled;
        visibility.toggle_visible_hidden();
    }
}

fn command_line_output(text: String) -> impl Bundle {
    (
        Name::new(format!("Message: {text}")),
        Node {
            height: px(24),
            ..default()
        },
        Text::new(text),
    )
}
