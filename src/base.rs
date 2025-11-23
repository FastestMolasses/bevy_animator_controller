use super::AnimatorController;
use bevy::{prelude::*, render::mesh::skinning::SkinnedMesh};

#[derive(Component)]
pub struct BoneIndex(pub usize);

#[derive(Debug, Clone, Copy)]
pub struct OzzTransform {
    pub scale: Vec3,
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
                    transform.scale = bone_trans[idx.0].scale;
                }
            }
        }
    }
}

pub(crate) fn add_bone_indexes(
    bones: Query<(Entity, &SkinnedMesh), Added<SkinnedMesh>>,
    parents: Query<&ChildOf>,
    controllers: Query<&AnimatorController>,
    names: Query<&Name>,
    mut commands: Commands,
) {
    for (entity, skinned_mesh) in &bones {
        // Find AnimatorController in ancestors
        let mut current_entity = entity;
        let mut skeleton = None;

        // Walk up to find controller
        loop {
            if let Ok(controller) = controllers.get(current_entity) {
                skeleton = Some(&controller.skeleton);
                break;
            }
            if let Ok(child_of) = parents.get(current_entity) {
                current_entity = child_of.parent();
            } else {
                break;
            }
        }

        if let Some(skeleton) = skeleton {
            let joint_names = skeleton.joint_names();
            for joint_entity in &skinned_mesh.joints {
                if let Ok(name) = names.get(*joint_entity) {
                    let mut found_index = None;
                    for (k, v) in joint_names.iter() {
                        if k == name.as_str() {
                            found_index = Some(*v);
                            break;
                        }
                    }

                    if let Some(index) = found_index {
                        commands
                            .entity(*joint_entity)
                            .insert(BoneIndex(index as usize));
                    }
                }
            }
        } else {
            for (i, joint_entity) in skinned_mesh.joints.iter().enumerate() {
                commands.entity(*joint_entity).insert(BoneIndex(i));
            }
        }
    }
}
