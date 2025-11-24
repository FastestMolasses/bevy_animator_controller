use bevy::prelude::*;
use bevy_animator_controller::{OzzAnimationPlugin, prelude::*};
use bevy_asset_loader::prelude::*;
use ozz_animation_rs::*;
use std::sync::Arc;

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    #[default]
    Loading,
    Playing,
}

#[derive(AssetCollection, Resource)]
pub struct PlayerAnimationAssets {
    #[asset(path = "greatsword_idle.ozz")]
    pub player_idle: Handle<OzzAsset>,
    #[asset(path = "skeleton.ozz")]
    pub skeleton: Handle<OzzAsset>,
    #[asset(path = "base_man.glb#Scene0")]
    pub player_mesh: Handle<Scene>,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, OzzAnimationPlugin))
        .init_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .load_collection::<PlayerAnimationAssets>()
                .continue_to_state(GameState::Playing),
        )
        .add_systems(OnEnter(GameState::Playing), (setup_scene,))
        // .add_systems(Update, update_debug_bone_transforms)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    player_animations: Res<PlayerAnimationAssets>,
    ozz_assets: ResMut<Assets<OzzAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut player = commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        SceneRoot(player_animations.player_mesh.clone()),
    ));

    let player_anim_controller = build_player_animation_controller(player_animations, ozz_assets);
    if let Some(controller) = player_anim_controller {
        player.insert(controller);
    }

    commands.spawn((
        Camera::default(),
        Camera3d::default(),
        Msaa::Off,
        Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::default() + Vec3::Z, Vec3::Y),
    ));

    // Sky
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.61, 0.98),
            unlit: true,
            cull_mode: None,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(400.0)),
    ));
}

pub(crate) fn build_player_animation_controller(
    player_animations: Res<PlayerAnimationAssets>,
    mut ozz_assets: ResMut<Assets<OzzAsset>>,
) -> Option<AnimatorController> {
    let skeleton = ozz_assets.get_mut(&player_animations.skeleton)?;
    let Ok(skeleton) = Skeleton::from_archive(&mut skeleton.archive) else {
        return None;
    };
    let skeleton = Arc::new(skeleton);

    let idle_anim = ozz_assets.get_mut(&player_animations.player_idle)?;
    let Ok(idle_anim) = Animation::from_archive(&mut idle_anim.archive) else {
        return None;
    };
    let idle_anim = Arc::new(idle_anim);

    let mut animation_layer = AnimationLayer::new(
        "Base Layer".to_string(),
        LayerBlendType::Override,
        1.0,
        &skeleton,
        "greatsword_idle".to_string(),
    );

    let idle_state = SimpleState::new(idle_anim, skeleton.num_soa_joints());
    animation_layer.add_state(
        "greatsword_idle".to_string(),
        AnimationState::Simple(idle_state),
    );

    let mut parameters = Parameters::new();
    parameters.set_float("speed", 0.0);

    Some(AnimatorController::new(
        skeleton.clone(),
        vec![animation_layer],
        parameters,
    ))
}
