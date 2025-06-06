use bevy::asset::{AssetLoader, AssetServer, Handle, LoadContext};
use bevy::asset::io::Reader;
use bevy::audio::AudioSource;
use bevy::image::{ImageLoaderSettings, ImageSampler};
use bevy::prelude::{Asset, Component, CubicCurve, FromWorld, Reflect, Resource, World};
use serde::{Deserialize, Serialize};
use bevy::math::Vec2;
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

#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect, Default)]
pub struct RaceTrack {
    pub track_name: String,
    pub points: Vec<Vec2>,
}

#[derive(Debug, Clone, Resource, Asset, Reflect, Deserialize, Default)]
pub struct TracksAsset {
    pub tracks: HashMap<String, RaceTrack>,
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