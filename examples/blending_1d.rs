use bevy::{
    prelude::*,
    render::{mesh::PrimitiveTopology, render_asset::RenderAssetUsages},
};
use bevy_animator_controller::{OzzAnimationPlugin, prelude::*};
use bevy_asset_loader::prelude::*;
use ozz_animation_rs::*;
use std::sync::{Arc, RwLock};

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    #[default]
    Loading,
    Playing,
}

#[derive(AssetCollection, Resource)]
pub struct PlayerAnimationAssets {
    #[asset(path = "greatsword_idle.ozz")]
    pub idle: Handle<OzzAsset>,
    #[asset(path = "simple_animation01.ozz")]
    pub run: Handle<OzzAsset>,
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
        .add_systems(OnEnter(GameState::Playing), setup_scene)
        .add_systems(Update, update_blend_parameter.run_if(in_state(GameState::Playing)))
        .run();
}

fn update_blend_parameter(
    time: Res<Time>,
    mut query: Query<&mut AnimatorController>,
) {
    for mut controller in query.iter_mut() {
        // Oscillate speed between 0.0 and 1.0
        let speed = (time.elapsed_secs().sin() + 1.0) / 2.0;
        controller.parameters_mut().set_float("speed", speed);
    }
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

    if let Some(controller) = build_blend_tree_controller(&player_animations, ozz_assets) {
        player.insert(controller);
    }

    commands.spawn((
        Camera::default(),
        Camera3d::default(),
        Msaa::Off,
        Transform::from_xyz(0.0, 1.5, 4.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // Light
    commands.spawn((
        PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            ..default()
        })),
    ));
}

fn build_blend_tree_controller(
    assets: &PlayerAnimationAssets,
    mut ozz_assets: ResMut<Assets<OzzAsset>>,
) -> Option<AnimatorController> {
    let skeleton = ozz_assets.get_mut(&assets.skeleton)?;
    let Ok(skeleton) = Skeleton::from_archive(&mut skeleton.archive) else {
        return None;
    };
    let skeleton = Arc::new(skeleton);

    let idle_anim = ozz_assets.get_mut(&assets.idle)?;
    let Ok(idle_anim) = Animation::from_archive(&mut idle_anim.archive) else {
        return None;
    };
    let idle_anim = Arc::new(idle_anim);

    let run_anim = ozz_assets.get_mut(&assets.run)?;
    let Ok(run_anim) = Animation::from_archive(&mut run_anim.archive) else {
        return None;
    };
    let run_anim = Arc::new(run_anim);

    // Create Blend Tree
    let idle_state = SimpleState::new(idle_anim.clone(), skeleton.num_soa_joints());
    let run_state = SimpleState::new(run_anim.clone(), skeleton.num_soa_joints());

    let motions = vec![
        MotionData {
            motion: BlendMotionState::Animation(Arc::new(RwLock::new(idle_state))),
            threshold: MotionThreshold::Simple1D(0.0),
        },
        MotionData {
            motion: BlendMotionState::Animation(Arc::new(RwLock::new(run_state))),
            threshold: MotionThreshold::Simple1D(1.0),
        },
    ];

    let blend_tree = BlendTree::new(
        &skeleton,
        BlendTreeType::Simple1D("speed".to_string()),
        motions,
    );

    // Create Layer containing the Blend Tree
    let mut animation_layer = AnimationLayer::new(
        "Locomotion".to_string(),
        LayerBlendType::Override,
        1.0,
        &skeleton,
        "blend_tree".to_string(),
    );

    animation_layer.add_state(
        "blend_tree".to_string(),
        AnimationState::Blend(BlendState::new(blend_tree)),
    );

    let mut parameters = Parameters::new();
    parameters.set_float("speed", 0.0);

    Some(AnimatorController::new(
        skeleton.clone(),
        vec![animation_layer],
        parameters,
    ))
}
