use bevy::{
    app::{App, Plugin},
    asset::{
        io::Reader,
        {Asset, AssetApp, AssetLoader, LoadContext},
    },
    prelude::*,
};
use ozz_animation_rs::{Archive, OzzError};
use std::io::Cursor;
use thiserror::Error;

/// An asset representing a loaded Ozz animation file
#[derive(TypePath, Asset)]
pub struct OzzAsset {
    pub archive: Archive<Cursor<Vec<u8>>>,
}

/// Plugin to load Ozz animation files
pub struct OzzAssetPlugin {
    extensions: Vec<&'static str>,
}

impl Plugin for OzzAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<OzzAsset>()
            .register_asset_loader(OzzAssetLoader {
                extensions: self.extensions.clone(),
            });
    }
}

impl OzzAssetPlugin {
    /// Create a new plugin that will load Ozz animation assets
    pub fn new(extensions: &[&'static str]) -> Self {
        Self {
            extensions: extensions.to_owned(),
        }
    }
}

/// Loads Ozz animation files
pub struct OzzAssetLoader {
    extensions: Vec<&'static str>,
}

/// Possible errors that can be produced by OzzAssetLoader
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum OzzLoaderError {
    /// An IO Error
    #[error("Could not read the file: {0}")]
    Io(#[from] std::io::Error),
    /// An Ozz Error
    #[error("Could not parse Ozz animation: {0}")]
    OzzError(#[from] OzzError),
}

impl AssetLoader for OzzAssetLoader {
    type Asset = OzzAsset;
    type Settings = ();
    type Error = OzzLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let archive = Archive::from_vec(bytes)?;
        Ok(OzzAsset { archive })
    }

    fn extensions(&self) -> &[&str] {
        &self.extensions
    }
}
