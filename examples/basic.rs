use bevy::{
    prelude::*,
    render::{mesh::PrimitiveTopology, render_asset::RenderAssetUsages},
};
use bevy_animator_controller::{OzzAnimationPlugin, prelude::*};
use bevy_asset_loader::prelude::*;
use ozz_animation_rs::*;
use std::sync::Arc;

const MAX_DEBUG_BONE_COUNT: usize = 64;

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

#[derive(Component)]
pub struct DebugBone;

pub(crate) fn setup_debug_bones(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let bone_mesh = meshes.add(build_bone_mesh());
    let bone_material = materials.add(Color::srgb(0.68, 0.68, 0.8));
    for i in 0..MAX_DEBUG_BONE_COUNT {
        commands.spawn((
            Mesh3d(bone_mesh.clone()),
            MeshMaterial3d(bone_material.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0),
            BoneIndex(i),
            DebugBone,
        ));
    }
}

fn update_debug_bone_transforms(
    mut query: Query<(&mut Transform, &mut Visibility, &BoneIndex), With<DebugBone>>,
    controller_query: Query<&AnimatorController, Without<BoneIndex>>,
) {
    let Ok(controller) = controller_query.single() else {
        return;
    };

    let bone_trans: &Vec<OzzTransform> = &controller.bone_trans;
    if !bone_trans.is_empty() {
        for (mut transform, mut visibility, idx) in query.iter_mut() {
            if idx.0 < bone_trans.len() {
                *visibility = Visibility::Visible;
                transform.translation = bone_trans[idx.0].position;
                transform.rotation = bone_trans[idx.0].rotation;
                transform.scale = bone_trans[idx.0].scale;
            }
        }
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

#[rustfmt::skip]
fn build_bone_mesh() -> Mesh {
    let c = [Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.2, 0.1, 0.1),
        Vec3::new(0.2, 0.1, -0.1),
        Vec3::new(0.2, -0.1, -0.1),
        Vec3::new(0.2, -0.1, 0.1),
        Vec3::new(0.0, 0.0, 0.0)];
    let n = [Vec3::cross(c[2] - c[1], c[2] - c[0]).normalize(),
        Vec3::cross(c[1] - c[2], c[1] - c[5]).normalize(),
        Vec3::cross(c[3] - c[2], c[3] - c[0]).normalize(),
        Vec3::cross(c[2] - c[3], c[2] - c[5]).normalize(),
        Vec3::cross(c[4] - c[3], c[4] - c[0]).normalize(),
        Vec3::cross(c[3] - c[4], c[3] - c[5]).normalize(),
        Vec3::cross(c[1] - c[4], c[1] - c[0]).normalize(),
        Vec3::cross(c[4] - c[1], c[4] - c[5]).normalize()];

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![
        c[0], c[2], c[1],
        c[5], c[1], c[2],
        c[0], c[3], c[2],
        c[5], c[2], c[3],
        c[0], c[4], c[3],
        c[5], c[3], c[4],
        c[0], c[1], c[4],
        c[5], c[4], c[1],
    ])
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![
        n[0], n[0], n[0],
        n[1], n[1], n[1],
        n[2], n[2], n[2],
        n[3], n[3], n[3],
        n[4], n[4], n[4],
        n[5], n[5], n[5],
        n[6], n[6], n[6],
        n[7], n[7], n[7],
    ])
}
