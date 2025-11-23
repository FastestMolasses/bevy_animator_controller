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
    /// Transforms for bones
    pub bone_trans: Vec<OzzTransform>,
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

        // Count the number of bones
        let mut bone_count = 0;
        for _ in 0..skeleton.num_joints() {
            bone_count += 1;
        }

        let mut controller = Self {
            layers,
            parameters,
            final_blending_job,
            bone_trans: Vec::with_capacity(bone_count),
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
        let skeleton = self.skeleton.clone();
        self.update_bones(&skeleton);
        Ok(())
    }

    #[inline]
    pub fn update_bones(&mut self, skeleton: &Skeleton) {
        self.bone_trans.clear();

        if let Ok(local_transforms) = self.final_blending_job.output().unwrap().read() {
            for i in 0..skeleton.num_joints() {
                let current_soa_index = i / 4;
                let current_lane = i % 4;

                let current_pos = Vec3::new(
                    local_transforms[current_soa_index].translation.x[current_lane],
                    local_transforms[current_soa_index].translation.y[current_lane],
                    local_transforms[current_soa_index].translation.z[current_lane],
                );

                let current_rot = Quat::from_xyzw(
                    local_transforms[current_soa_index].rotation.x[current_lane],
                    local_transforms[current_soa_index].rotation.y[current_lane],
                    local_transforms[current_soa_index].rotation.z[current_lane],
                    local_transforms[current_soa_index].rotation.w[current_lane],
                );

                let current_scale = Vec3::new(
                    local_transforms[current_soa_index].scale.x[current_lane],
                    local_transforms[current_soa_index].scale.y[current_lane],
                    local_transforms[current_soa_index].scale.z[current_lane],
                );

                self.bone_trans.push(OzzTransform {
                    scale: current_scale,
                    rotation: current_rot,
                    position: current_pos,
                });
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

    #[inline]
    pub fn parameters_mut(&mut self) -> &mut Parameters {
        &mut self.parameters
    }
}
