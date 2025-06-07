//! Spawn the main level.

use crate::racing::{ControlPoints, CurrentTrack, Curves, RaceTrack, Racing, Shooting, TrackPart, TracksAsset, TracksAssetLoader};
use crate::{
    asset_tracking::LoadResource,
    audio::music,
    game::player::{PlayerAssets, player},
    screens::Screen,
};
use avian2d::PhysicsPlugins;
use avian2d::prelude::{Collider, Gravity, PhysicsDebugPlugin, RigidBody};
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::basic::GRAY;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy_enhanced_input::EnhancedInputPlugin;
use bevy_enhanced_input::prelude::{InputContext, InputContextAppExt};
use crate::game::player::Player;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        PhysicsPlugins::default(), 
        PhysicsDebugPlugin::default(),
                     EnhancedInputPlugin))
        .add_input_context::<Racing>()
        .add_input_context::<Shooting>()
        .insert_resource(Gravity::ZERO)
        .init_resource::<CurrentTrack>()
        .init_asset::<TracksAsset>()
        .init_asset_loader::<TracksAssetLoader>()
        .register_type::<LevelAssets>()
        .load_resource::<LevelAssets>()
        .add_systems(PostUpdate, follow_camera.before(TransformSystem::TransformPropagate).run_if(in_state(Screen::Gameplay)))
    ;
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct LevelAssets {
    #[dependency]
    music: Handle<AudioSource>,
    #[dependency]
    track: Handle<TracksAsset>,
}

impl FromWorld for LevelAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            music: assets.load("audio/music/Fluffing A Duck.ogg"),
            track: assets.load("race.tracks"),
        }
    }
}

/// A system that spawns the main level.
pub fn spawn_level(
    mut commands: Commands,
    level_assets: Res<LevelAssets>,
    player_assets: Res<PlayerAssets>,
    mut track_assets: ResMut<Assets<TracksAsset>>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut current_track: ResMut<CurrentTrack>,
) {
    let tracks = track_assets.get_mut(&level_assets.track).unwrap();
    current_track.0 = tracks.get_next_track().cloned();

    commands.spawn((
        Name::new("Level"),
        Transform::default(),
        Visibility::default(),
        StateScoped(Screen::Gameplay),
        children![
            player(200.0, &player_assets, &mut texture_atlas_layouts),
            (
                Name::new("Gameplay Music"),
                music(level_assets.music.clone())
            ),
        ],
    ));
}

pub fn follow_camera(
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Camera>)>,
) {
    if let Ok(mut camera_transform) = camera_query.single_mut() {
        if let Ok(player_transform) = player_query.single() {
            camera_transform.translation = player_transform.translation;
        }
    }
}

pub fn instantiate_track(
    current_track: Res<CurrentTrack>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut mesh_query: Query<Entity, With<TrackPart>>,
) {
    if current_track.0.is_none() || !current_track.is_changed() {
        return;
    }
    
    let track = current_track.0.as_ref().unwrap();

    for mesh in mesh_query.iter_mut() {
        commands.entity(mesh).despawn();
    }
    
    let bounds = track.get_bounds();

    for (i, (p0, p1)) in bounds.iter().enumerate() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        /*
        our triangles are, maybe?
        p2---p3
        p0
             p3
        p0---p1


         */
        let (p2, p3) = if i == bounds.len() - 1 {
            &bounds[0]
        } else {
            &bounds[i + 1]
        };

        let vertices = vec![
            [p0.x, p0.y, 0.0], //0
            [p1.x, p1.y, 0.0], //1
            [p2.x, p2.y, 0.0], //2
            [p3.x, p3.y, 0.0], //3
        ];
        let color = LinearRgba::from(GRAY);
        let colors = vertices
            .iter()
            .map(|_| [color.red, color.green, color.blue, color.alpha])
            .collect::<Vec<_>>();

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

        let indices = vec![
            0, 2, 3,
            0, 1, 3];
        mesh.insert_indices(Indices::U32(indices));
        
        commands.spawn((
            TrackPart,
            RigidBody::Static,
            Collider::convex_hull(vec![*p0, *p2, *p3, *p1]).unwrap(),
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(materials.add(Color::from(GRAY))),
        ));
    }
}

/// This system uses gizmos to draw the current [`Curves`] by breaking it up into a large number
/// of line segments.
fn draw_curve(curve: Res<Curves>, mut gizmos: Gizmos) {
    let Some(ref center_curve) = curve.0 else {
        return;
    };
    // Scale resolution with curve length so it doesn't degrade as the length increases.
    let resolution = 100 * center_curve.segments().len();
    //Modify this to insert race track sections!
    gizmos.linestrip(
        center_curve
            .iter_positions(resolution)
            .map(|pt| pt.extend(0.0)),
        Color::srgb(1.0, 1.0, 1.0),
    );
}
