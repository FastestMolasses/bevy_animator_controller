use super::AnimatorController;
use bevy::{prelude::*, render::mesh::skinning::SkinnedMesh};

#[derive(Component)]
pub struct BoneIndex(pub usize);

#[derive(Debug, Clone, Copy)]
pub struct OzzTransform {
    pub scale: f32,
    pub rotation: Quat,
    pub position: Vec3,
}

pub fn animate_bones(mut controller_query: Query<&mut AnimatorController>, time: Res<Time>) {
    for mut controller in controller_query.iter_mut() {
        let _ = controller.update(&time);
    }
}

pub(crate) fn update_bone_transforms(
    mut query: Query<(&mut Transform, &BoneIndex)>,
    controller_query: Query<&AnimatorController, Without<BoneIndex>>,
) {
    for controller in controller_query.iter() {
        let bone_trans = &controller.bone_trans;
        if !bone_trans.is_empty() {
            for (mut transform, idx) in query.iter_mut() {
                if idx.0 < bone_trans.len() {
                    transform.translation = bone_trans[idx.0].position;
                    transform.rotation = bone_trans[idx.0].rotation;
                    // transform.scale = Vec3::splat(bone_trans[idx.0].scale);
                }
            }
        }
    }
}

pub(crate) fn add_bone_indexes(bones: Query<&SkinnedMesh, Added<SkinnedMesh>>, mut commands: Commands) {
    for skinned_mesh in &bones {
        for (joint_index, joint_entity) in skinned_mesh.joints.iter().enumerate() {
            commands
                .entity(*joint_entity)
                .insert(BoneIndex(joint_index));
        }
    }
}
