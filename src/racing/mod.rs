use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AssetServer, Handle, LoadContext};
use bevy::audio::AudioSource;
use bevy::image::{ImageLoaderSettings, ImageSampler};
use bevy::math::{Vec2, vec2};
use bevy::platform::collections::HashMap;
use bevy::prelude::{
    Asset, Component, CubicCardinalSpline, CubicCurve, CyclicCubicGenerator, FromWorld, Reflect,
    Resource, World,
};
use bevy_enhanced_input::prelude::{InputAction, InputContext};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const RESOLUTION: usize = 5;

#[derive(Component)]
pub struct TrackPart;

/// The curve presently being displayed. This is optional because there may not be enough control
/// points to actually generate a curve.
#[derive(Clone, Default, Resource)]
pub struct Curves(pub Option<CubicCurve<Vec2>>);

/// The control points used to generate a curve. The tangent components are only used in the case of
/// Hermite interpolation.
#[derive(Clone, Resource)]
pub struct ControlPoints {
    pub points: Vec<Vec2>,
    pub selected: Option<usize>,
}

#[derive(Debug, Clone, Resource, Default)]
pub struct CurrentTrack(pub Option<RaceTrack>);

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct RaceTrack {
    pub track_name: String,
    pub points: Vec<Vec2>,
}

impl RaceTrack {
    pub fn form_curve(&self) -> Curves {
        let points = self.points.iter().copied();
        let spline = CubicCardinalSpline::new_catmull_rom(points);

        Curves(spline.to_curve_cyclic().ok())
    }

    pub fn get_bounds(&self) -> Vec<(Vec2, Vec2)> {
        let mut normals = Vec::new();
        let tension = 0.5;
        let binding = self.form_curve();
        let track_curve = binding.0.as_ref().unwrap();
        let resolution = RESOLUTION * track_curve.segments().len();
        let track_curve = track_curve.iter_positions(resolution).collect::<Vec<_>>();

        for i in 0..track_curve.len() {
            let tangent = if i == 0 {
                // Forward difference at start
                (track_curve[i + 1] - track_curve[i]) * tension * 2.0
            } else if i == track_curve.len() - 1 {
                // Backward difference at end
                (track_curve[i] - track_curve[i - 1]) * tension * 2.0
            } else {
                // Central difference for internal points
                (track_curve[i + 1] - track_curve[i - 1]) * tension
            };

            let tangent = tangent.normalize_or_zero();

            let normal = tangent.rotate(Vec2::from_angle(std::f32::consts::PI / -2.0)) * 20.0; // 90° rotation
            let normal2 = normal.rotate(Vec2::from_angle(std::f32::consts::PI));

            normals.push((track_curve[i] + normal, track_curve[i] + normal2));
        }
        normals
    }
}

impl Default for RaceTrack {
    fn default() -> Self {
        Self {
            track_name: String::new(),
            points: vec![vec2(-500., -200.), vec2(-500., -150.)],
        }
    }
}

#[derive(Debug, Clone, Resource, Asset, Reflect, Deserialize, Serialize)]
pub struct TracksAsset {
    pub tracks: Vec<RaceTrack>,
    pub current_track_index: Option<usize>,
}

impl Default for TracksAsset {
    fn default() -> Self {
        let mut asset = Self {
            tracks: Vec::new(),
            current_track_index: None,
        };
        let _ = asset.new_track();
        asset
    }
}

impl TracksAsset {
    pub fn new_track(&mut self) {
        let name = format!("Track {}", self.tracks.len() + 1);
        let track = RaceTrack {
            track_name: name,
            points: vec![vec2(-500., -200.), vec2(-500., -150.)],
        };
        self.store_track(track);
    }

    pub fn update_current_track(&mut self, points: Vec<Vec2>) {
        if let Some(mut track) = self.get_current_track_mut() {
            track.points = points;
        }
    }

    pub fn store_track(&mut self, track: RaceTrack) {
        self.tracks.push(track);
        self.current_track_index = Some(self.tracks.len() - 1);
    }

    pub fn delete_current_track(&mut self) {
        match self.current_track_index {
            None => return,
            Some(index) => {
                self.tracks.remove(index);
                self.current_track_index = None;
            }
        }
    }

    pub fn get_current_track_mut(&mut self) -> Option<&mut RaceTrack> {
        match self.current_track_index {
            None => None,
            Some(index) => self.tracks.get_mut(index),
        }
    }

    pub fn get_current_track(&self) -> Option<&RaceTrack> {
        match self.current_track_index {
            None => None,
            Some(index) => self.tracks.get(index),
        }
    }

    pub fn get_next_track(&mut self) -> Option<&RaceTrack> {
        match self.current_track_index {
            None => {
                self.current_track_index = Some(0);
                self.tracks.get(0)
            }
            Some(index) => {
                if index == self.tracks.len() - 1 {
                    self.current_track_index = Some(0);
                    self.tracks.get(0)
                } else {
                    self.current_track_index = Some(index + 1);
                    self.tracks.get(index + 1)
                }
            }
        }
    }

    pub fn get_prev_track(&mut self) -> Option<&RaceTrack> {
        match self.current_track_index {
            None => {
                self.current_track_index = Some(0);
                self.tracks.get(0)
            }
            Some(index) => {
                if index == 0 {
                    self.current_track_index = Some(self.tracks.len() - 1);
                    self.tracks.get(self.tracks.len() - 1)
                } else {
                    self.current_track_index = Some(index - 1);
                    self.tracks.get(index - 1)
                }
            }
        }
    }
}

#[derive(Default)]
pub struct TracksAssetLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TracksAssetLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    /// A [JSON](json) Error
    #[error("Could not parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl AssetLoader for TracksAssetLoader {
    type Asset = TracksAsset;
    type Settings = ();
    type Error = TracksAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let custom_asset = serde_json::from_slice(&bytes).unwrap_or_default();

        Ok(custom_asset)
    }

    fn extensions(&self) -> &[&str] {
        &["tracks"]
    }
}


#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Forward;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Reverse;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Left;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Right;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Fire;

#[derive(InputContext)]
pub struct Racing;
#[derive(InputContext)]
pub struct Shooting;
