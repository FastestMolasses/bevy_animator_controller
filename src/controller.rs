use super::{AnimationLayer, LayerBlendType, OzzTransform, Parameters};
use bevy::prelude::*;
use ozz_animation_rs::*;
use std::sync::{Arc, RwLock};

#[derive(Component, Debug)]
pub struct AnimatorController {
    /// Animation layers
    layers: Vec<AnimationLayer>,
    /// Parameters for the animator
    parameters: Parameters,
    /// Final blending job
    final_blending_job: BlendingJobArc,
    /// Local to model job
    l2m_job: LocalToModelJobArc,
    /// Output of the local to model job
    pub models: Arc<RwLock<Vec<Mat4>>>,
    /// Transforms for bones
    pub bone_trans: Vec<OzzTransform>,
    /// Transforms for spines
    pub spine_trans: Vec<OzzTransform>,
    /// Skeleton
    pub skeleton: Arc<Skeleton>,
}

unsafe impl Send for AnimatorController {}
unsafe impl Sync for AnimatorController {}

impl AnimatorController {
    #[inline]
    pub fn new(
        skeleton: Arc<Skeleton>,
        layers: Vec<AnimationLayer>,
        parameters: Parameters,
    ) -> Self {
        // Setup blending job
        let mut final_blending_job: BlendingJobArc = BlendingJob::default();
        let blending_output = Arc::new(RwLock::new(vec![
            SoaTransform::default();
            skeleton.num_soa_joints()
        ]));
        final_blending_job.set_skeleton(skeleton.clone());
        final_blending_job.set_output(blending_output.clone());

        // Setup local to model job
        let mut l2m_job: LocalToModelJobArc = LocalToModelJob::default();
        l2m_job.set_skeleton(skeleton.clone());
        let models = Arc::new(RwLock::new(vec![Mat4::default(); skeleton.num_joints()]));
        l2m_job.set_output(Arc::new(RwLock::new(
            models.read().unwrap()
            .iter()
            .map(|m| glam::Mat4::from_cols_array_2d(&m.to_cols_array_2d()))
            .collect()
        )));
        l2m_job.set_input(blending_output);

        // Count the number of bones and spines
        let mut bone_count = 0;
        let mut spine_count = 0;
        for i in 0..skeleton.num_joints() {
            let parent_id = skeleton.joint_parent(i);
            if parent_id as i32 == SKELETON_NO_PARENT {
                continue;
            }
            bone_count += 1;
            spine_count += 1;
            if skeleton.is_leaf(i as i16) {
                spine_count += 1;
            }
        }

        let mut controller = Self {
            layers,
            parameters,
            final_blending_job,
            l2m_job,
            models,
            bone_trans: Vec::with_capacity(bone_count),
            spine_trans: Vec::with_capacity(spine_count),
            skeleton,
        };
        controller
            .build_blending_layers()
            .expect("Failed to build blending layers");
        controller
    }

    #[inline]
    pub fn add_layer(&mut self, layer: AnimationLayer) {
        self.layers.push(layer);
    }

    #[inline]
    pub fn update(&mut self, time: &Time) -> Result<(), OzzError> {
        // TODO: STATE UPDATES CAN BE PARALLELIZED
        // Update all layers
        for (index, layer) in self.layers.iter_mut().enumerate() {
            layer.update(time, &mut self.parameters)?;

            // Only update the input pointer if the output source has changed
            if layer.has_output_changed() {
                // There will always be a blending layer at the same index as the animation layer being updated
                self.final_blending_job.layers_mut()[index].transform = layer.get_output_pointer();
                layer.clear_output_changed();
            }
        }

        // Reset triggers after update
        self.parameters.reset_triggers();

        self.final_blending_job.run()?;
        // self.l2m_job.run()?;
        let skeleton = self.skeleton.clone();
        self.update_bones(&skeleton);
        Ok(())
    }

    #[inline]
    fn update_bones_old(&mut self, skeleton: &Skeleton) {
        self.bone_trans.clear();
        self.spine_trans.clear();

        // The transformation matrices are extracted from the output of the LocalToModelJob
        // and used to calculate the bone and spine transformations
        let modals = self.models.buf().unwrap();
        for (i, current) in modals.iter().enumerate() {
            let parent_id = skeleton.joint_parent(i);
            if parent_id as i32 == SKELETON_NO_PARENT {
                continue;
            }
            let parent = &modals[parent_id as usize];

            let current_pos = current.w_axis.xyz();
            let parent_pos = parent.w_axis.xyz();
            // Scale is calculated as the distance between the current joint and its parent
            let scale: f32 = (current_pos - parent_pos).length();

            // Normalized direction vector from the parent joint to the current joint
            let bone_dir = (current_pos - parent_pos).normalize();

            // This part determines a binormal vector:
            // It compares the dot products of bone_dir with the parent's x and z axes.
            // The axis that's more perpendicular to bone_dir is chosen as the binormal.
            let dot1 = Vec3::dot(bone_dir, parent.x_axis.xyz());
            let dot2 = Vec3::dot(bone_dir, parent.z_axis.xyz());
            let binormal = if dot1.abs() < dot2.abs() {
                parent.x_axis.xyz()
            } else {
                parent.z_axis.xyz()
            };

            // Here, an orthonormal basis is constructed:
            // bone_rot_y is perpendicular to both binormal and bone_dir.
            // bone_rot_z is perpendicular to both bone_dir and bone_rot_y.
            // These vectors form a rotation matrix, which is then converted to a quaternion
            let bone_rot_y = Vec3::cross(binormal, bone_dir).normalize();
            let bone_rot_z = Vec3::cross(bone_dir, bone_rot_y).normalize();
            let bone_rot = Quat::from_mat3(&Mat3::from_cols(bone_dir, bone_rot_y, bone_rot_z));

            self.bone_trans.push(OzzTransform {
                scale,
                rotation: bone_rot,
                position: parent_pos,
            });

            let parent_rot = Quat::from_mat4(parent);
            self.spine_trans.push(OzzTransform {
                scale,
                rotation: parent_rot,
                position: parent_pos,
            });

            if skeleton.is_leaf(i as i16) {
                let current_rot = Quat::from_mat4(current);
                self.spine_trans.push(OzzTransform {
                    scale,
                    rotation: current_rot,
                    position: current_pos,
                });
            }
        }
    }

    pub fn update_bones(&mut self, skeleton: &Skeleton) {
        self.bone_trans.clear();
        self.spine_trans.clear();

        if let Ok(local_transforms) = self.final_blending_job.output().unwrap().read() {
            for i in 0..skeleton.num_joints() {
                let parent_id = skeleton.joint_parent(i);
                if parent_id as i32 == SKELETON_NO_PARENT {
                    continue;
                }

                let current_soa_index = i / 4;
                let current_lane = i % 4;
                let parent_soa_index = parent_id as usize / 4;
                let parent_lane = parent_id as usize % 4;

                let current_pos = Vec3::new(
                    local_transforms[current_soa_index].translation.x[current_lane],
                    local_transforms[current_soa_index].translation.y[current_lane],
                    local_transforms[current_soa_index].translation.z[current_lane],
                );

                let parent_pos = Vec3::new(
                    local_transforms[parent_soa_index].translation.x[parent_lane],
                    local_transforms[parent_soa_index].translation.y[parent_lane],
                    local_transforms[parent_soa_index].translation.z[parent_lane],
                );

                let current_rot = Quat::from_xyzw(
                    local_transforms[current_soa_index].rotation.x[current_lane],
                    local_transforms[current_soa_index].rotation.y[current_lane],
                    local_transforms[current_soa_index].rotation.z[current_lane],
                    local_transforms[current_soa_index].rotation.w[current_lane],
                );

                let parent_rot = Quat::from_xyzw(
                    local_transforms[parent_soa_index].rotation.x[parent_lane],
                    local_transforms[parent_soa_index].rotation.y[parent_lane],
                    local_transforms[parent_soa_index].rotation.z[parent_lane],
                    local_transforms[parent_soa_index].rotation.w[parent_lane],
                );

                let scale = (current_pos - parent_pos).length();

                self.bone_trans.push(OzzTransform {
                    scale,
                    rotation: current_rot,
                    position: current_pos,
                });

                self.spine_trans.push(OzzTransform {
                    scale,
                    rotation: parent_rot,
                    position: parent_pos,
                });

                if skeleton.is_leaf(i as i16) {
                    self.spine_trans.push(OzzTransform {
                        scale,
                        rotation: current_rot,
                        position: current_pos,
                    });
                }
            }
        }
    }

    #[inline]
    pub fn build_blending_layers(&mut self) -> Result<(), OzzError> {
        // Collect layer data to avoid borrow checker issues
        let layer_data = self
            .layers
            .iter()
            .map(|l| (l.layer_blend_type, l.layer_weight, l.get_output_pointer()));

        // Blend all layers together
        self.final_blending_job.layers_mut().clear();
        let mut base_added = false;
        for (blend_type, weight, transform) in layer_data {
            match blend_type {
                LayerBlendType::Override => {
                    if !base_added {
                        self.final_blending_job.layers_mut().push(BlendingLayer {
                            transform,
                            weight,
                            joint_weights: vec![],
                        });
                        base_added = true;
                    } else {
                        self.final_blending_job.layers_mut().push(BlendingLayer {
                            transform,
                            weight,
                            joint_weights: vec![],
                        });
                    }
                }
                LayerBlendType::Additive => {
                    // Handle additive blending when implemented
                    // return Err("Additive blending not yet implemented".into());
                    panic!("Additive blending not yet implemented");
                }
            }
        }

        Ok(())
    }
}
