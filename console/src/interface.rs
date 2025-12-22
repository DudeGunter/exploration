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
    let font_size = 12.0;
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
                    height: px(font_size + 4 as f32),
                    ..default()
                },
                TextInputNode {
                    mode: TextInputMode::SingleLine,
                    clear_on_submit: true,
                    is_enabled: false,
                    ..default()
                },
                TextFont {
                    font_size,
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

pub fn console_output(text: String) -> impl Bundle {
    let font_size = 12.0;
    (
        Name::new(format!("Message: {text}")),
        Node {
            min_height: px(font_size),
            ..default()
        },
        // Removed for now, visual bug can be simply fixed but I think this looks cleaner generally
        //BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        Text::new(text),
        TextFont {
            font_size,
            ..default()
        },
    )
}
