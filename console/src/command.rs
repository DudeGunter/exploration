use crate::*;

#[derive(Event, Reflect, Deref, Clone)]
pub struct TryCommand(pub String);

pub fn handle_command(
    trigger: On<TryCommand>,
    mut commands: Commands,
    mut message_writer: MessageWriter<SpawnReflected>,
    console_message_container: Single<Entity, With<ConsoleMessageContainer>>,
) {
    let (cmd, args) = trigger
        .event()
        .split_once(' ')
        .unwrap_or((trigger.event(), ""));
    let mut outputs: Vec<Entity> = Vec::new();
    let mut out = |message: &str| {
        // lil func action
        outputs.push(
            commands
                .spawn(command_line_output(message.to_string()))
                .id(),
        );
    };
    match cmd {
        "help" => {
            out("spawn: spawn <Component>");
            out("\tThe <Component> must #[reflect(Component, Default)]");
        }
        "spawn" => {
            out("Attempting to spawn...");
            message_writer.write(SpawnReflected(args.to_string()));
        }
        "67" => out("67"),
        _ => out("Unknown command"),
    }
    commands
        .entity(console_message_container.entity())
        .add_children(outputs.as_slice());
}

/// Attempts to spawn an entity based off the given type string.
/// The type must #[reflect(Component, Default)]
/// Uses direct world access which is taxing, but allowed here as a dev tool
/// FYI: direct world access doesn't allow any other process to run in parallel
#[derive(Message, Deref)]
pub struct SpawnReflected(String);

pub fn spawn_reflected(world: &mut World) {
    let events: Vec<SpawnReflected> = world
        .resource_mut::<Messages<SpawnReflected>>()
        .drain()
        .collect();

    for event in events {
        let component_to_insert = {
            let registry = world.get_resource::<AppTypeRegistry>().unwrap().read();
            registry
                .get_with_short_type_path(event.as_str())
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
            output = format!("Entity {} spawned successfully.", event.0);
            output_color = Color::srgb(0.4, 1.0, 0.4);
        } else {
            // This could be refined to provide more detail potentially
            output = "DNE or missing ReflectDefault and/or ReflectComponent".to_string();
            output_color = Color::srgb(1.0, 0.4, 0.4);
        }

        // This spawning logic could be refined, although the dir world access makes it annoying
        let command_line = world
            .spawn((command_line_output(output), TextColor(output_color)))
            .id();
        let container = world
            .query::<(Entity, &ConsoleMessageContainer)>()
            .single(world)
            .unwrap()
            .0;
        world.commands().entity(container).add_child(command_line);
    }
}
