//! The screen state for the main gameplay.

use crate::screens::Screen;
use bevy::{
    gizmos::gizmos::Gizmos,
    input::{mouse::MouseButtonInput, ButtonState},
    math::{cubic_splines::*, vec2},
    prelude::*,
};
use std::fs;
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::basic::GRAY;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use crate::racing::{ControlPoints, Curves, RaceTrack, TracksAsset, TrackPart};

pub(super) fn plugin(app: &mut App) {
    app
        .add_systems(OnEnter(Screen::Editor), setup_editor)
        .add_systems(
            Update,
            (
                handle_keypress,
                handle_mouse_move,
                handle_mouse_press,
                draw_edit_move,
                update_curve,
                draw_curve,
                draw_control_points,
            )
                .chain()
                .run_if(in_state(Screen::Editor)),
        );
}

pub fn setup_editor(mut commands: Commands) {
    // Initialize the modes with their defaults:
    
    // Starting data for [`ControlPoints`]:
    let default_points = vec![
        vec2(-500., -200.),
        vec2(-500., -150.)
    ];
    
    let default_control_data = ControlPoints {
        points: default_points.into_iter().collect(),
        selected: None,
    };

    let curve = form_curve(&default_control_data);
    commands.insert_resource(curve);
    commands.insert_resource(default_control_data);

    // Mouse tracking information:
    commands.insert_resource(MousePosition::default());
    commands.insert_resource(MouseEditMove::default());
    commands.insert_resource(MouseMoveMove::default());

    // The instructions and modes are rendered on the left-hand side in a column.
    let instructions_text = "Click and drag to add control points\n\
        R: Remove the selected control point\n\
        Arrows: Change selected control point\n\
        S: Save track.json\n\
        L: Load track.json";
    let style = TextFont::default();

    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(20.0),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((Text::new(instructions_text), style.clone()));
        });
}

// -----------------------------------
// Curve-related Resources and Systems
// -----------------------------------

/// This system is responsible for updating the [`Curves`] when the [control points] or active modes
/// change.
///
/// [control points]: ControlPoints
fn update_curve(
    control_points: Res<ControlPoints>,
    mut commands: Commands,
    mut curve: ResMut<Curves>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut mesh_query: Query<Entity, With<TrackPart>>,
) {
    if !control_points.is_changed() {
        return;
    }

    for mesh in mesh_query.iter_mut() {
        commands.entity(mesh).despawn();
    }


    let points =
        control_points.points.iter().copied();
    let spline = CubicCardinalSpline::new_catmull_rom(points);
    
    *curve = Curves(spline.to_curve_cyclic().ok());
    let track_curve = curve.0.as_ref().unwrap();
    let resolution = 100 * track_curve.segments().len();
    let track_curve = track_curve.iter_positions(resolution)
    //     .map(|pt| { 
    //     // Here is where we create our polygons, our normals, etc. 
    //     pt.extend(0.0) 
    // })
        .collect::<Vec<_>>();
    
    let bounds = compute_bounds(&track_curve);
    
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
        let (p2, p3) = if i == bounds.len() - 1 {&bounds[0]} else {&bounds[i + 1]};
        
        let vertices = vec![
            [p0.x, p0.y, 0.0], //0
            [p1.x, p1.y, 0.0], //1
            [p2.x, p2.y, 0.0], //2
            [p3.x, p3.y, 0.0], //3
        ];
        let color = LinearRgba::from(GRAY);
        let colors = vertices.iter().map(|_| 
        [color.red, color.green, color.blue, color.alpha]
        ).collect::<Vec<_>>();
        
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

        let indices = vec![
            0, 2, 3,
            0,1,3
        ];
        mesh.insert_indices(Indices::U32(indices));

        commands.spawn((
                           TrackPart,
                           Mesh2d(meshes.add(mesh)),
                       MeshMaterial2d(materials.add(Color::from(GRAY)))
                           ,)
        );

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
        center_curve.iter_positions(resolution).map(|pt| pt.extend(0.0)),
        Color::srgb(1.0, 1.0, 1.0),
    );
}

/// This system uses gizmos to draw the current [control points] as circles, displaying their
/// tangent vectors as arrows in the case of a Hermite spline.
///
/// [control points]: ControlPoints
fn draw_control_points(
    control_points: Res<ControlPoints>,
    mut gizmos: Gizmos,
) {
    for (i, point) in control_points.points.iter().enumerate() { 
        if Some(i) == control_points.selected {
            gizmos.circle_2d(*point, 10.0, Color::srgb(1.0, 0.0, 0.0));
           
        } else {
            gizmos.circle_2d(*point, 10.0, Color::srgb(0.0, 1.0, 0.0));
        }
    }
}

/// Helper function for generating a [`Curves`] from [control points] and selected modes.
///
/// [control points]: ControlPoints
fn form_curve(
    control_points: &ControlPoints
) -> Curves {
    let points =
        control_points.points.iter().copied();
    let spline = CubicCardinalSpline::new_catmull_rom(points);
    
    Curves(spline.to_curve_cyclic().ok())
}

pub fn compute_bounds(
    control_points: &[Vec2],
) -> Vec<(Vec2,Vec2)> {
    let mut normals = Vec::new();
    let tension = 0.5;

    for i in 0..control_points.len() {
        let tangent = if i == 0 {
            // Forward difference at start
            (control_points[i + 1] - control_points[i]) * tension * 2.0
        } else if i == control_points.len() - 1 {
            // Backward difference at end
            (control_points[i] - control_points[i - 1]) * tension * 2.0
        } else {
            // Central difference for internal points
            (control_points[i + 1] - control_points[i - 1]) * tension
        };

        let tangent = tangent.normalize_or_zero();

        let normal = tangent.rotate(Vec2::from_angle(std::f32::consts::PI / -2.0)) * 20.0; // 90° rotation
        let normal2 = normal.rotate(Vec2::from_angle(std::f32::consts::PI));

        normals.push((control_points[i] + normal, control_points[i] + normal2)); 
    }

    normals
}

// -----------------------------------
// Input-related Resources and Systems
// -----------------------------------

/// A small state machine which tracks a click-and-drag motion used to create new control points.
///
/// When the user is not doing a click-and-drag motion, the `start` field is `None`. When the user
/// presses the left mouse button, the location of that press is temporarily stored in the field.
#[derive(Clone, Default, Resource)]
struct MouseEditMove {
    start: Option<Vec2>,
}

#[derive(Clone, Default, Resource)]
struct MouseMoveMove {
    start: Option<Vec2>,
}

/// The current mouse position, if known.
#[derive(Clone, Default, Resource)]
struct MousePosition(Option<Vec2>);

/// Update the current cursor position and track it in the [`MousePosition`] resource.
fn handle_mouse_move(
    mut cursor_events: EventReader<CursorMoved>,
    mut mouse_position: ResMut<MousePosition>,
) {
    if let Some(cursor_event) = cursor_events.read().last() {
        mouse_position.0 = Some(cursor_event.position);
    }
}

/// This system handles updating the [`MouseEditMove`] resource, orchestrating the logical part
/// of the click-and-drag motion which actually creates new control points.
fn handle_mouse_press(
    mut button_events: EventReader<MouseButtonInput>,
    mouse_position: Res<MousePosition>,
    mut edit_move: ResMut<MouseEditMove>,
    mut move_move: ResMut<MouseMoveMove>,
    mut control_points: ResMut<ControlPoints>,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    let Some(mouse_pos) = mouse_position.0 else {
        return;
    };

    // Handle click and drag behavior
    for button_event in button_events.read() {
        match button_event.button {
            MouseButton::Left => {
                match button_event.state {
                    ButtonState::Pressed => {
                        if edit_move.start.is_some() {
                            // If the edit move already has a start, press event should do nothing.
                            continue;
                        }
                        // This press represents the start of the edit move.
                        edit_move.start = Some(mouse_pos);
                    }

                    ButtonState::Released => {
                        // Release is only meaningful if we started an edit move.
                        let Some(start) = edit_move.start else {
                            continue;
                        };

                        let (camera, camera_transform) = *camera;

                        // Convert the starting point and end point (current mouse pos) into world coords:
                        let Ok(point) = camera.viewport_to_world_2d(camera_transform, start) else {
                            continue;
                        };
                        // The start of the click-and-drag motion represents the point to add,
                        // while the difference with the current position represents the tangent.
                        control_points.points.push(point);

                        // Reset the edit move since we've consumed it.
                        edit_move.start = None;
                    }
                }
            },
            MouseButton::Right => {
                if control_points.selected.is_none() {
                    continue;
                }
                match button_event.state {
                    ButtonState::Pressed => {
                        if move_move.start.is_some() {
                            // If the edit move already has a start, press event should do nothing.
                            continue;
                        }
                        // This press represents the start of the edit move.
                        move_move.start = Some(mouse_pos);
                    }

                    ButtonState::Released => {
                        // Release is only meaningful if we started an edit move.
                        let Some(start) = move_move.start else {
                            continue;
                        };

                        let (camera, camera_transform) = *camera;

                        // Convert the starting point and end point (current mouse pos) into world coords:
                        let Ok(point) = camera.viewport_to_world_2d(camera_transform, start) else {
                            continue;
                        };
                        // The start of the click-and-drag motion represents the point to add,
                        // while the difference with the current position represents the tangent.
                        let selected = control_points.selected.unwrap();
                        let to_mutate = control_points.points.get_mut(selected).unwrap();
                        *to_mutate = point;

                        // Reset the edit move since we've consumed it.
                        move_move.start = None;
                    }
                }
            }
                _ => continue,
        }
    }
}

/// This system handles drawing the "preview" control point based on the state of [`MouseEditMove`].
fn draw_edit_move(
    edit_move: Res<MouseEditMove>,
    mouse_position: Res<MousePosition>,
    mut gizmos: Gizmos,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    let Some(start) = edit_move.start else {
        return;
    };
    let Some(mouse_pos) = mouse_position.0 else {
        return;
    };

    let (camera, camera_transform) = *camera;

    // Resources store data in viewport coordinates, so we need to convert to world coordinates
    // to display them:
    let Ok(start) = camera.viewport_to_world_2d(camera_transform, start) else {
        return;
    };
    let Ok(end) = camera.viewport_to_world_2d(camera_transform, mouse_pos) else {
        return;
    };

    gizmos.circle_2d(start, 10.0, Color::srgb(0.0, 1.0, 0.7));
    gizmos.circle_2d(start, 7.0, Color::srgb(0.0, 1.0, 0.7));
    gizmos.arrow_2d(start, end, Color::srgb(1.0, 0.0, 0.7));
}

/// This system handles all keyboard commands.
fn handle_keypress(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut control_points: ResMut<ControlPoints>,
) {
    // R => remove last control point
    if keyboard.just_pressed(KeyCode::KeyR) {
        if control_points.selected.is_some() {
            let selected = control_points.selected.unwrap();
            control_points.points.remove(selected);
            control_points.selected = None;
        } else {
            control_points.points.pop();
        }

    }
    if keyboard.just_pressed(KeyCode::KeyS) {
        save_to_file(&control_points, "assets/1.track.json");
    }
    if keyboard.just_pressed(KeyCode::KeyL) {
       let race_track = load_from_file("assets/1.track.json");
        control_points.points = race_track.points;
    }
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        if control_points.selected.is_none() {
            control_points.selected = Some(0);
        } else {
            let mut current = control_points.selected.unwrap();
            if current == 0 {
                current = control_points.points.len() - 1;
            } else {
                current -= 1;
            }
            control_points.selected = Some(current);
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        if control_points.selected.is_none() {
            control_points.selected = Some(0);
        } else {
            let mut current = control_points.selected.unwrap();
            if current >= control_points.points.len() - 1 {
                current = 0;
            } else {
                current += 1;
            }
            control_points.selected = Some(current);
        }
    }
}

fn save_to_file(data: &ControlPoints, path: &str) {
    let race_track=RaceTrack {
        track_name: "Test Track".to_string(),
        points: data.points.clone(),
    };
    let json = serde_json::to_string_pretty(&race_track).unwrap();
    fs::write(path, json).unwrap();
}

fn load_from_file(path: &str) -> RaceTrack {
    let contents = fs::read_to_string(path).unwrap();
    serde_json::from_str(&contents).unwrap()
}
