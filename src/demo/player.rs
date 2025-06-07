//! Player-specific behavior.

use avian2d::prelude::{CoefficientCombine, Collider, ColliderDensity, Friction, Restitution};
use bevy_enhanced_input::prelude::{Fired, Ongoing};
use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};
use bevy_enhanced_input::prelude::{ActionState, Actions};
use KeyCode::*;
use crate::{
    AppSystems, PausableSystems,
    asset_tracking::LoadResource,
    demo::{
        animation::PlayerAnimation,
        movement::{MovementController, ScreenWrap},
    },
};
use crate::racing::*;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<Player>();

    app.register_type::<PlayerAssets>();
    app.load_resource::<PlayerAssets>();

    // Record directional input as movement controls.
    app
    //     .add_systems(
    //     Update,
    //     record_player_directional_input
    //         .in_set(AppSystems::RecordInput)
    //         .in_set(PausableSystems),
    // )
        .add_observer(accelerate)
    ;
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
    // The action will trigger when space or gamepad south button is pressed.
    racing_actions.bind::<Forward>().to(KeyW);//, GamepadButton::RightTrigger2));
    racing_actions.bind::<Reverse>().to(KeyS);//, GamepadButton::LeftTrigger2));
    racing_actions.bind::<Left>().to(KeyA);//, GamepadAxis::LeftStickX));
    racing_actions.bind::<Right>().to(KeyD);//, GamepadAxis::LeftStickX));
    let mut shooting_actions = Actions::<Shooting>::default();
    shooting_actions.bind::<Fire>().to(Space);//, GamepadButton::South));
    
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
        Collider::rectangle(2.0, 5.0),
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        ColliderDensity(2.0),
        Transform::from_scale(Vec2::splat(8.0).extend(1.0)),
        // player_animation,
    )
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
struct Player;

fn record_player_directional_input(
    input: Res<ButtonInput<KeyCode>>,
    mut controller_query: Query<&mut MovementController, With<Player>>,
) {
    // Collect directional input.
    let mut intent = Vec2::ZERO;
    if input.pressed(KeyW) || input.pressed(KeyCode::ArrowUp) {
        intent.y += 1.0;
    }
    if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown) {
        intent.y -= 1.0;
    }
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft) {
        intent.x -= 1.0;
    }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight) {
        intent.x += 1.0;
    }

    // Normalize intent so that diagonal movement is the same speed as horizontal / vertical.
    // This should be omitted if the input comes from an analog stick instead.
    let intent = intent.normalize_or_zero();

    // Apply movement intent to controllers.
    for mut controller in &mut controller_query {
        controller.intent = intent;
    }
}

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
fn accelerate(trigger: Trigger<Ongoing<Forward>>, mut transforms: Query<&mut Transform, With<Player>>) {
    let mut transform = transforms.get_mut(trigger.target()).unwrap();

    // Move to the camera direction.
    let rotation = transform.rotation;
}