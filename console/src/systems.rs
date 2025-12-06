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
    console_config.insert_command_with_metadata(
        "set_field",
        CommandMetadata {
            description: "Set a field on a singleton entity. Requires exclusive World access"
                .to_string(),
            usage: "set_field <component> <field> <value>".to_string(),
        },
        set_field,
    );
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

use bevy::reflect::{ReflectMut, serde::ReflectDeserializer};
use serde::de::DeserializeSeed;

fn set_field(In(arguments): In<String>, world: &mut World) {
    let parts: Vec<&str> = arguments.split_whitespace().collect();

    if parts.len() < 3 {
        world.trigger(ConsoleMessage::new("Usage: <component> <field> <value>"));
        return;
    }

    let component_name = parts[0];
    let field_name = parts[1];
    let value = parts[2..].join(" ");

    let reflect_component = {
        let registry = world.get_resource::<AppTypeRegistry>().unwrap().read();
        registry
            .get_with_short_type_path(component_name)
            .and_then(|registration| registration.data::<ReflectComponent>().cloned())
    };

    let reflect_component = match reflect_component {
        Some(rc) => rc,
        None => {
            world.trigger(ConsoleMessage::new(format!(
                "Component '{}' not found or doesn't support reflection",
                component_name
            )));
            return;
        }
    };

    // Find entity with this component and mutate it
    let mut entity_iter = world.query::<Entity>();
    let entities: Vec<Entity> = entity_iter.iter(&world).collect();

    let mut found = false;
    for entity in entities {
        if let Ok(entity_mut) = world.get_entity_mut(entity) {
            if reflect_component.contains(&entity_mut) {
                world.resource_scope(|world, registry: Mut<AppTypeRegistry>| {
                    let registry = registry.read();
                    if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                        if let Some(mut reflected) = reflect_component.reflect_mut(&mut entity_mut)
                        {
                            // Match on the ReflectMut enum to get the Struct variant
                            if let ReflectMut::Struct(struct_mut) = reflected.reflect_mut() {
                                if let Some(field) = struct_mut.field_mut(field_name) {
                                    // Deserialize the string value as RON - wrap in parentheses for tuple syntax
                                    let ron_str = format!("({})", value);
                                    if let Ok(mut deserializer) =
                                        ron::Deserializer::from_str(&ron_str)
                                    {
                                        let reflect_deserializer =
                                            ReflectDeserializer::new(&registry);
                                        if let Ok(new_val) =
                                            reflect_deserializer.deserialize(&mut deserializer)
                                        {
                                            // Apply the new value to the field
                                            field.apply(new_val.as_partial_reflect());
                                            world.trigger(ConsoleMessage::new(format!(
                                                "Set {}.{} = {}",
                                                component_name, field_name, value
                                            )));
                                            found = true;
                                        } else {
                                            world.trigger(ConsoleMessage::new(format!(
                                                "Failed to deserialize value for field '{}'",
                                                field_name
                                            )));
                                        }
                                    }
                                } else {
                                    world.trigger(ConsoleMessage::new(format!(
                                        "Field '{}' not found on {}",
                                        field_name, component_name
                                    )));
                                }
                            } else {
                                world.trigger(ConsoleMessage::new(format!(
                                    "Component '{}' is not a struct",
                                    component_name
                                )));
                            }
                        }
                    }
                });
                if found {
                    break;
                }
            }
        }
    }

    if !found {
        world.trigger(ConsoleMessage::new(format!(
            "No entity with component '{}' found",
            component_name
        )));
    }
}
