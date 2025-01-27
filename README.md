# Bevy Animator Controller

A 3D animator controller for Bevy based on the Unity Animator Controller, built on top of [ozz-animation-rs](https://github.com/SlimeYummy/ozz-animation-rs).

## Features

- Animation layers
- Animation parameters
- Blend trees
- States
- Transitions
- Ozz asset loader

## Example

```rs
use bevy::prelude::*;
use bevy_animator_controller::prelude::*;

// Using `bevy_asset_loader` to load assets
// Can be done through Bevy normally too
#[derive(AssetCollection, Resource)]
pub struct PlayerAnimationAssets {
    #[asset(path = "greatsword_idle.ozz")]
    pub player_idle: Handle<OzzAsset>,
    #[asset(path = "skeleton.ozz")]
    pub skeleton: Handle<OzzAsset>,
    #[asset(path = "base_man.glb#Scene0")]
    pub player_mesh: Handle<Scene>,
}

fn build_player_animation_controller(
    player_animations: &Res<PlayerAnimationAssets>,
    mut ozz_assets: ResMut<Assets<OzzAsset>>,
) -> Option<AnimatorController> {
    // Load skeleton
    let skeleton = ozz_assets.get_mut(&player_animations.skeleton)?;
    let Ok(skeleton) = Skeleton::from_archive(&mut skeleton.archive) else {
        return None;
    };
    let skeleton = Arc::new(skeleton);

    // Load idle animation
    let idle_anim = ozz_assets.get_mut(&player_animations.player_idle)?;
    let Ok(idle_anim) = Animation::from_archive(&mut idle_anim.archive) else {
        return None;
    };
    let idle_anim = Arc::new(idle_anim);

    // Construct the animation controller
    let mut animation_layer = AnimationLayer::new(
        "Base Layer".to_string(),       // Layer name
        LayerBlendType::Override,       // Layer blend type
        1.0,                            // Layer weight
        &skeleton,                      // Skeleton
        "greatsword_idle".to_string(),  // Default state name
    );

    let idle_state = SimpleState::new(idle_anim, skeleton.num_soa_joints());
    animation_layer.add_state(
        "greatsword_idle".to_string(),      // State name
        AnimationState::Simple(idle_state), // State type
    );

    // Add parameters
    let mut parameters = Parameters::new();
    parameters.set_float("speed", 0.0);

    Some(AnimatorController::new(
        skeleton.clone(),
        vec![animation_layer],
        parameters,
    ))
}
```

## License

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

## Credits

The example model is taken from [Turbosquid](https://www.turbosquid.com/3d-models/slender-man-lores-base-mesh-3d-model-2236602).
