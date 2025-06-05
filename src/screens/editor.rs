//! The screen state for the main gameplay.

use crate::screens::Screen;
use bevy::{
    gizmos::gizmos::Gizmos,
    input::{ButtonState, mouse::MouseButtonInput},
    math::{cubic_splines::*, vec2},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::fs;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Editor), setup_editor)
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
        vec2(-250., 250.),
        vec2(250., 250.),
        vec2(500., -200.),
    ];
    
    let default_control_data = ControlPoints {
        points: default_points.into_iter().collect(),
    };

    let curve = form_curve(&default_control_data);
    commands.insert_resource(curve);
    commands.insert_resource(default_control_data);

    // Mouse tracking information:
    commands.insert_resource(MousePosition::default());
    commands.insert_resource(MouseEditMove::default());

    // The instructions and modes are rendered on the left-hand side in a column.
    let instructions_text = "Click and drag to add control points and their tangents\n\
        R: Remove the last control point\n\
        S: Cycle the spline construction being used\n\
        C: Toggle cyclic curve construction";
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

/// The current spline mode, which determines the spline method used in conjunction with the
/// control points.
#[derive(Clone, Copy, Resource, Default)]
enum SplineMode {
    #[default]
    Hermite,
    Cardinal,
    B,
}

impl std::fmt::Display for SplineMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SplineMode::Hermite => f.write_str("Hermite"),
            SplineMode::Cardinal => f.write_str("Cardinal"),
            SplineMode::B => f.write_str("B"),
        }
    }
}

/// The current cycling mode, which determines whether the control points should be interpolated
/// cyclically (to make a loop).
#[derive(Clone, Copy, Resource, Default)]
enum CyclingMode {
    #[default]
    NotCyclic,
    Cyclic,
}

impl std::fmt::Display for CyclingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CyclingMode::NotCyclic => f.write_str("Not Cyclic"),
            CyclingMode::Cyclic => f.write_str("Cyclic"),
        }
    }
}

/// The curve presently being displayed. This is optional because there may not be enough control
/// points to actually generate a curve.
#[derive(Clone, Default, Resource)]
struct Curves {
    pub inner_curve: Option<CubicCurve<Vec2>>,
    pub center_curve: Option<CubicCurve<Vec2>>,
    pub outer_curve: Option<CubicCurve<Vec2>>,
}

/// The control points used to generate a curve. The tangent components are only used in the case of
/// Hermite interpolation.
#[derive(Clone, Resource)]
struct ControlPoints {
    pub points: Vec<Vec2>,
}

/// This system is responsible for updating the [`Curves`] when the [control points] or active modes
/// change.
///
/// [control points]: ControlPoints
fn update_curve(
    control_points: Res<ControlPoints>,
    mut curve: ResMut<Curves>,
) {
    if !control_points.is_changed() {
        return;
    }

    *curve = form_curve(&control_points);
}

/// This system uses gizmos to draw the current [`Curves`] by breaking it up into a large number
/// of line segments.
fn draw_curve(curve: Res<Curves>, mut gizmos: Gizmos) {
    let Some(ref curve) = curve.center_curve else {
        return;
    };
    // Scale resolution with curve length so it doesn't degrade as the length increases.
    let resolution = 100 * curve.segments().len();
    //Modify this to insert race track sections!
    gizmos.linestrip(
        curve.iter_positions(resolution).map(|pt| pt.extend(0.0)),
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
    for &point in &control_points.points {
        gizmos.circle_2d(point, 10.0, Color::srgb(0.0, 1.0, 0.0));
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
    Curves {
        inner_curve: None,
        center_curve: spline.to_curve_cyclic().ok(),
        outer_curve: None,
    }
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
    mut control_points: ResMut<ControlPoints>,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    let Some(mouse_pos) = mouse_position.0 else {
        return;
    };

    // Handle click and drag behavior
    for button_event in button_events.read() {
        if button_event.button != MouseButton::Left {
            continue;
        }

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
        control_points.points.pop();
    }
}

fn save_map() {}

fn save_to_file(data: &RaceTrack, path: &str) {
    let json = serde_json::to_string_pretty(data).unwrap();
    fs::write(path, json).unwrap();
}

fn load_from_file(path: &str) -> RaceTrack {
    let contents = fs::read_to_string(path).unwrap();
    serde_json::from_str(&contents).unwrap()
}

#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct RaceTrack {
    pub track_name: String,
    pub points_and_tangents: Vec<(Vec2, Vec2)>,
}
