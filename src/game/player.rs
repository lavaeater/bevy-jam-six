//! Player-specific behavior.

use crate::ReflectComponent;
use crate::ReflectResource;
use crate::racing::{Fire, GameLayer, Move, Racing, Shooting};
use crate::{
    PausableSystems,
    asset_tracking::LoadResource,
    game::{
        animation::PlayerAnimation,
        movement::{MovementController, ScreenWrap},
    },
    racing,
};
use avian2d::prelude::{AngularDamping, Collider, ColliderDensity, CollisionLayers, ExternalForce, ExternalTorque, Friction, LinearDamping, LinearVelocity, MaxAngularSpeed, MaxLinearSpeed, Restitution, RigidBody};
use bevy::prelude::KeyCode::*;
use bevy::prelude::{Name, Query, Res, Time, Trigger, Vec2, With};
use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::{
        App, Asset, AssetServer, Assets, AudioSource, Bundle, Component, FromWorld, Handle, Image,
        Reflect, Resource, TextureAtlasLayout, Transform, UVec2, World,
    },
};
use bevy_enhanced_input::prelude::{Actions, Cardinal, Fired, Input};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<Player>();

    app.register_type::<PlayerAssets>();
    app.load_resource::<PlayerAssets>();

    // Record directional input as movement controls.
    app.add_observer(apply_steering);
}

/// The player character.
pub fn player(
    max_speed: f32,
    player_assets: &PlayerAssets,
    texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
) -> impl Bundle {
    // A texture atlas is a way to split a single image into a grid of related images.
    // You can learn more in this example: https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 6, 2, Some(UVec2::splat(1)), None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    let player_animation = PlayerAnimation::new();

    /*
    Controls, bitch
     */
    let mut racing_actions = Actions::<Racing>::default();
    racing_actions
        .bind::<racing::Move>()
        .to((Cardinal::wasd_keys()));
    let mut shooting_actions = Actions::<Shooting>::default();
    shooting_actions.bind::<Fire>().to(Space); //, GamepadButton::South));

    (
        Name::new("Player"),
        racing_actions,
        shooting_actions,
        Player,
        // Sprite {
        //     image: player_assets.ducky.clone(),
        //     texture_atlas: Some(TextureAtlas {
        //         layout: texture_atlas_layout,
        //         index: player_animation.get_atlas_index(),
        //     }),
        //     ..default()
        // },
        // CollisionLayers::new(
        //     GameLayer::Player,
        // [GameLayer::Default, GameLayer::Obstacle]),
        RigidBody::Dynamic,
        Collider::rectangle(2.0, 3.5),
        ExternalForce::default(),
        ExternalTorque::default(),
        Transform::from_scale(Vec2::splat(8.0).extend(1.0)),
        ColliderDensity(0.1),
        // MaxLinearSpeed(max_speed),
        // MaxAngularSpeed(50.),
        LinearDamping(0.5),
        AngularDamping(2.0),
    )
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
pub struct Player;

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct PlayerAssets {
    #[dependency]
    ducky: Handle<Image>,
    #[dependency]
    pub steps: Vec<Handle<AudioSource>>,
}

impl FromWorld for PlayerAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            ducky: assets.load_with_settings(
                "images/ducky.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
            steps: vec![
                assets.load("audio/sound_effects/step1.ogg"),
                assets.load("audio/sound_effects/step2.ogg"),
                assets.load("audio/sound_effects/step3.ogg"),
                assets.load("audio/sound_effects/step4.ogg"),
            ],
        }
    }
}

// Apply movemenet when `Move` action considered fired.
fn apply_steering(
    trigger: Trigger<Fired<Move>>,
    mut player_query: Query<(&mut ExternalForce, &mut ExternalTorque, &Transform), With<Player>>,
) {
    if let Ok((mut ext_force, mut ext_torque, transform)) = player_query.get_mut(trigger.target()) {
        let direction = Vec2::new(transform.right().x, transform.right().y);

        let v = trigger.value;

        let v = direction.rotate(v);
        
        ext_force
            .apply_force(v * 500.0)
            .with_persistence(false);
        
        // ext_torque.apply_torque(-trigger.value.x * 100.0)
        //     .with_persistence(false);
    }
}

fn control_car(
    mut query: Query<(&mut LinearVelocity, &Transform), With<Player>>,
    time: Res<Time>,
) {
    if let Ok((mut velocity, transform)) = query.single_mut() {
        
    }
    let dt = time.delta_secs();

    let forward = transform.rotation.mul_vec3(Vec3::Y).truncate(); // car's forward vector

    let speed = velocity.linvel.dot(forward);
    let mut acceleration = Vec2::ZERO;
    let turn = if keyboard_input.pressed(KeyCode::A) {
        1.0
    } else if keyboard_input.pressed(KeyCode::D) {
        -1.0
    } else {
        0.0
    };

    // Throttle/brake
    if keyboard_input.pressed(KeyCode::W) {
        acceleration += forward * 10.0;
    }
    if keyboard_input.pressed(KeyCode::S) {
        acceleration -= forward * 10.0;
    }

    // Turning with skidding
    let skidding = speed.abs() > 2.0;
    let turn_rate = if skidding { 1.5 } else { 3.0 };

    velocity.angvel = turn as f32 * turn_rate * speed.signum();

    // Apply acceleration
    velocity.linvel += acceleration * dt;

    // Simulate lateral friction (reduce sideways velocity)
    let right = Vec2::new(forward.y, -forward.x); // perpendicular
    let lateral_speed = velocity.linvel.dot(right);
    velocity.linvel -= right * lateral_speed * 0.8; // damping for slide
}
