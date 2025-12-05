use bevy::prelude::*;
use bevy_ui_text_input::*;

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

pub fn command_line_output(text: String) -> impl Bundle {
    (
        Name::new(format!("Message: {text}")),
        Node {
            height: px(24),
            ..default()
        },
        // Removed for now, visual bug can be simply fixed but I think this looks cleaner generally
        //BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        Text::new(text),
    )
}
