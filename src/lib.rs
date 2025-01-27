mod asset_loader;
mod base;
mod blend_tree;
mod controller;
mod layer;
mod parameters;
mod state;

pub mod prelude;
pub use prelude::*;

use bevy::{app::Animation, prelude::*};

pub struct OzzAnimationPlugin;

impl Plugin for OzzAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(OzzAssetPlugin::new(&["ozz"]))
            .add_systems(Update, add_bone_indexes)
            .add_systems(
                PostUpdate,
                ((animate_bones, update_bone_transforms)
                    .before(bevy::render::mesh::inherit_weights)
                    .ambiguous_with_all())
                .in_set(Animation)
                .before(TransformSystem::TransformPropagate),
            );
    }
}
