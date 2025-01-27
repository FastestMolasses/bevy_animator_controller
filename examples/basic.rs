use bevy::{
    prelude::*,
    render::{mesh::PrimitiveTopology, render_asset::RenderAssetUsages},
};
use bevy_animator_controller::prelude::*;

const MAX_DEBUG_BONE_COUNT: usize = 64;

fn main() {}

#[derive(Component)]
pub struct DebugBone;

pub(crate) fn setup_debug_bones(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Debug Bones
    let bone_mesh = meshes.add(build_bone_mesh());
    let bone_material = materials.add(Color::srgb(0.68, 0.68, 0.8));
    for i in 0..MAX_DEBUG_BONE_COUNT {
        // commands.spawn((
        //     Mesh3d(bone_mesh.clone()),
        //     MeshMaterial3d(bone_material.clone()),
        //     Transform::from_xyz(0.0, 0.0, 0.0),
        //     Visibility::Hidden,
        //     BoneIndex(i),
        //     DebugBone,
        // ));
    }
}

pub(crate) fn update_debug_bone_transforms(
    mut query: Query<(&mut Transform, &mut Visibility, &BoneIndex), With<DebugBone>>,
    controller_query: Query<(&AnimatorController, &Transform), (With<Player>, Without<BoneIndex>)>,
) {
    for (controller, player_transform) in controller_query.iter() {
        let bone_trans = &controller.bone_trans;
        if !bone_trans.is_empty() {
            for (mut transform, mut visibility, idx) in query.iter_mut() {
                if idx.0 < bone_trans.len() {
                    *visibility = Visibility::Visible;
                    transform.translation =
                        player_transform.translation + bone_trans[idx.0].position;
                    transform.rotation = bone_trans[idx.0].rotation;
                    // transform.scale = Vec3::splat(bone_trans[idx.0].scale);
                } else {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}

pub fn spawn_base_man(mut commands: Commands, player_assets: Res<PlayerAnimationAssets>) {
    commands.spawn((
        Transform::from_xyz(0.0, 5.0, 0.0),
        SceneRoot(player_assets.player_mesh.clone()),
    ));
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
