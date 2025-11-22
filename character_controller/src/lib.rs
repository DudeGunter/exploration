use avian3d::prelude::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// This should all be implemented with the idea of a networking!!!

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Bundle, Serialize, Deserialize)]
pub struct CharacterControllerBundle {
    character_controller: CharacterController,
    collider: Collider,
    rigidbody: RigidBody,
    position: Transform,
}

impl Default for CharacterControllerBundle {
    fn default() -> Self {
        Self {
            character_controller: CharacterController::default(),
            collider: Collider::capsule(0.5, 1.0),
            rigidbody: RigidBody::Dynamic,
            position: Transform::default(),
        }
    }
}

#[derive(Component, Debug, Reflect, Clone, Copy, Serialize, Deserialize)]
pub struct CharacterController {
    pub speed: f32,
    pub jump_force: f32,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            speed: 5.0,
            jump_force: 10.0,
        }
    }
}
