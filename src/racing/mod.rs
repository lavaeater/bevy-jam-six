use bevy::asset::{AssetLoader, AssetServer, Handle, LoadContext};
use bevy::asset::io::Reader;
use bevy::audio::AudioSource;
use bevy::image::{ImageLoaderSettings, ImageSampler};
use bevy::prelude::{Asset, Component, CubicCurve, FromWorld, Reflect, Resource, World};
use serde::{Deserialize, Serialize};
use bevy::math::{vec2, Vec2};
use bevy::platform::collections::HashMap;
use thiserror::Error;


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

#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect)]
pub struct RaceTrack {
    pub track_name: String,
    pub points: Vec<Vec2>,
}

impl Default for RaceTrack {
    fn default() -> Self {
        Self {
            track_name: String::new(),
            points: vec![
                vec2(-500., -200.),
                vec2(-500., -150.)
            ]
        }
    }
}

#[derive(Debug, Clone, Resource, Asset, Reflect, Deserialize, Serialize)]
pub struct TracksAsset {
    pub tracks: HashMap<String, RaceTrack>,
    pub current_track_key: Option<String>
}

impl Default for TracksAsset {
    fn default() -> Self {
        let mut asset = Self {
            tracks: HashMap::new(),
            current_track_key: None
        };
        let _ = asset.new_track();
        asset
    }
    
}

impl TracksAsset {
    pub fn new_track(&mut self) -> RaceTrack {
        let name = format!("Track {}", self.tracks.len() + 1);
        let track = RaceTrack {
            track_name: name,
            points: vec![
                vec2(-500., -200.),
                vec2(-500., -150.)
            ]
        };
        self.store_track(track.clone());
        self.current_track_key = Some(track.track_name.clone());
        track
    }
    
    pub fn update_current_track(&mut self, points: Vec<Vec2>) {
        if let Some(track) = &mut self.current_track {
            track.points = points;
        }
        self.store_track(self.current_track.clone().unwrap());
    }

    pub fn store_track(&mut self, track: RaceTrack) {
        self.tracks.insert(track.track_name.clone(), track);
        self.tracks.keys().(&track.track_name);
    }
    
    pub fn delete_track(&mut self, track_name: String) {
        self.tracks.remove(&track_name);
    }
    
    pub fn get_next_track(&mut self) -> Option<RaceTrack> {
        let current_index = self.current_track_index.unwrap_or(0);
        let next_index = (current_index + 1) % self.tracks.len();
        self.current_track_index = Some(next_index);
        self.tracks.values().nth(next_index).cloned()
    }
    
    pub fn get_prev_track(&mut self) -> Option<RaceTrack> {
        let current_index = self.current_track_index.unwrap_or(0);
        let prev_index = if current_index == 0 { self.tracks.len() - 1 } else { current_index - 1 };
        self.current_track_index = Some(prev_index);
        self.tracks.values().nth(prev_index).cloned()
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