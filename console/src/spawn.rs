use crate::*;

pub fn add_spawn_command(mut console_config: ResMut<ConsoleConfig>) {
    console_config.insert_command("spawn", spawn_reflected);
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
