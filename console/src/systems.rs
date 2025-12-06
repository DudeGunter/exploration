use crate::*;

pub fn default_commands(mut console_config: ResMut<ConsoleConfig>) {
    console_config.insert_command_with_metadata(
        "help",
        CommandMetadata {
            description: "Display help information".to_string(),
            usage: "help [command]".to_string(),
        },
        help,
    );
    console_config.insert_command("67", |_: In<String>, mut commands: Commands| {
        commands.trigger(ConsoleMessage::new("67"))
    });
    console_config.insert_command("spawn", spawn_reflected);
}

pub fn help(In(argument): In<String>, console_config: Res<ConsoleConfig>, mut commands: Commands) {
    info!("Help command ran with the argument: {}", argument);
    if argument.is_empty() {
        for command in console_config.get_commands() {
            match console_config.get_metadata(command) {
                Some(metadata) => {
                    commands.trigger(ConsoleMessage::new(format!("Command: {}", command)));
                    commands.trigger(ConsoleMessage::new(format!(
                        "   ->Description: {}",
                        metadata.description
                    )));
                    commands.trigger(ConsoleMessage::new(format!(
                        "   ->Usage: {}",
                        metadata.usage
                    )));
                }
                None => info!("Command not found: {}", command),
            }
        }
    } else {
        match console_config.get_metadata(argument.clone()) {
            Some(metadata) => {
                commands.trigger(ConsoleMessage::new(format!("Command: {}", argument)));
                commands.trigger(ConsoleMessage::new(format!(
                    "   ->Description: {}",
                    metadata.description
                )));
                commands.trigger(ConsoleMessage::new(format!(
                    "   ->Usage: {}",
                    metadata.usage
                )));
            }
            None => commands.trigger(ConsoleMessage::new(format!(
                "Command not found: {}",
                argument
            ))),
        }
    }
}

pub fn spawn_reflected(In(component): In<String>, world: &mut World) {
    info!("Attempting to spawn component: {}", component);
    let component_to_insert = {
        let registry = world.get_resource::<AppTypeRegistry>().unwrap().read();
        registry
            .get_with_short_type_path(component.as_str())
            .and_then(|registration| {
                let reflect_component = registration.data::<ReflectComponent>()?.clone();
                let reflect_default = registration.data::<ReflectDefault>()?;
                Some((reflect_component, reflect_default.default()))
            })
    };

    let output: String;
    let output_color: Color;
    if let Some((reflect_component, new_component)) = component_to_insert {
        world.resource_scope(|world, registry: Mut<AppTypeRegistry>| {
            let registry = registry.read();
            let mut entity = world.spawn_empty();
            reflect_component.insert(&mut entity, new_component.as_ref(), &registry);
        });
        output = format!("Entity {} spawned successfully.", component);
        output_color = Color::srgb(0.4, 1.0, 0.4);
    } else {
        // This could be refined to provide more detail potentially
        output = "DNE or missing ReflectDefault and/or ReflectComponent".to_string();
        output_color = Color::srgb(1.0, 0.4, 0.4);
    }

    world.trigger(crate::protocol::ConsoleMessage {
        message: output,
        color: output_color,
    });
}
