use super::{BlendState, Parameters, SimpleState};
use bevy::prelude::*;
use ozz_animation_rs::{
    BlendingJob, BlendingJobArc, BlendingLayer, OzzError, Skeleton, SoaTransform,
};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub enum BlendTreeType {
    /// 1 directional blending, given a single parameter name
    Simple1D(String),
    /// 2 directional blending, given 2 parameters names
    Directional2D(String, String),
}

#[derive(Debug)]
pub struct BlendTree {
    blend_type: BlendTreeType,
    motions: Vec<MotionData>,
    blend_job: BlendingJobArc,
    output: Arc<RwLock<Vec<SoaTransform>>>,
}

/// Represents a motion threshold for blending depending on the type of blend tree
#[derive(Debug)]
pub enum MotionThreshold {
    /// 1D threshold with a single value
    Simple1D(f32),
    /// 2D threshold with 2 values
    Directional2D(f32, f32),
}

#[derive(Debug)]
pub enum BlendMotionState {
    Animation(Arc<RwLock<SimpleState>>),
    SubTree(Arc<RwLock<BlendState>>),
}

#[derive(Debug)]
pub struct MotionData {
    pub motion: BlendMotionState,
    pub threshold: MotionThreshold,
}

impl BlendTree {
    pub fn new(
        skeleton: &Arc<Skeleton>,
        blend_type: BlendTreeType,
        motions: Vec<MotionData>,
    ) -> Self {
        let mut blend_job = BlendingJob::default();
        blend_job.set_skeleton(skeleton.clone());
        let output = Arc::new(RwLock::new(vec![
            SoaTransform::default();
            skeleton.num_soa_joints()
        ]));
        blend_job.set_output(output.clone());

        let mut tree = BlendTree {
            blend_type,
            motions,
            blend_job,
            output,
        };
        tree.build_blend_layers();
        tree
    }

    #[inline(always)]
    pub fn build_blend_layers(&mut self) {
        self.blend_job.layers_mut().clear();
        for motion_data in &self.motions {
            let output_pointer = match motion_data.motion {
                BlendMotionState::Animation(ref state) => match state.read() {
                    Ok(state) => state.get_output_pointer(),
                    Err(_) => continue,
                },
                BlendMotionState::SubTree(ref state) => match state.read() {
                    Ok(state) => state.get_output_pointer(),
                    Err(_) => continue,
                },
            };

            self.blend_job.layers_mut().push(BlendingLayer {
                transform: output_pointer,
                weight: 0.0,
                joint_weights: vec![],
            });
        }
    }

    #[inline(always)]
    pub fn update(&mut self, time: &Time, params: &mut Parameters) -> Result<(), OzzError> {
        // Calculate weights based on parameters
        match &self.blend_type {
            BlendTreeType::Simple1D(param_name) => {
                if let Some(value) = params.get_float(param_name) {
                    self.calculate_weights_1d(value);
                }
            }
            BlendTreeType::Directional2D(x_param, y_param) => {
                let x_value = params.get_float(x_param);
                let y_value = params.get_float(y_param);
                if let (Some(x), Some(y)) = (x_value, y_value) {
                    self.calculate_weights_2d(x, y);
                }
            }
        }

        // TODO: STATE UPDATES CAN BE PARALLELIZED
        // Update motion states and blend layers
        for (i, motion_data) in self.motions.iter_mut().enumerate() {
            // Dont update inactive animations
            if self.blend_job.layers_mut()[i].weight == 0.0 {
                continue;
            }

            match &motion_data.motion {
                BlendMotionState::Animation(state) => {
                    if let Ok(mut state) = state.write() {
                        state.update(time)?;
                    }
                }
                BlendMotionState::SubTree(state) => {
                    if let Ok(mut state) = state.write() {
                        state.update(time, params)?;
                    }
                }
            }
        }

        // Run the blending job
        self.blend_job.run()?;
        Ok(())
    }

    #[inline(always)]
    fn calculate_weights_1d(&mut self, param_value: f32) {
        if self.motions.is_empty() {
            return;
        }
        let blend_layers = self.blend_job.layers_mut();

        if self.motions.len() == 1 {
            blend_layers[0].weight = 1.0;
            return;
        }

        // Find which 2 values to blend between
        for i in 0..self.motions.len() - 1 {
            let MotionThreshold::Simple1D(current_threshold) = self.motions[i].threshold else {
                continue;
            };
            let MotionThreshold::Simple1D(next_threshold) = self.motions[i + 1].threshold else {
                continue;
            };

            if param_value >= current_threshold && param_value <= next_threshold {
                let t = (param_value - current_threshold) / (next_threshold - current_threshold);
                let weight = 1.0 - t;
                blend_layers[i].weight = if weight.abs() < f32::EPSILON {
                    0.0
                } else {
                    weight
                };
                blend_layers[i + 1].weight = t;
            } else {
                blend_layers[i].weight = 0.0;
            }
        }

        // Edge cases if the first or last motion exceeds the threshold
        if let MotionThreshold::Simple1D(first_threshold) = self.motions[0].threshold {
            if param_value <= first_threshold {
                blend_layers[0].weight = 1.0;
            }
        } else if let Some(last_motion) = self.motions.last_mut()
            && let MotionThreshold::Simple1D(last_threshold) = last_motion.threshold
            && param_value >= last_threshold
            && let Some(last_layer) = blend_layers.last_mut()
        {
            last_layer.weight = 1.0
        }
    }

    fn calculate_weights_2d(&mut self, x_param_value: f32, y_param_value: f32) {
        // Reset all weights to 0 initially
        for layer in self.blend_job.layers_mut() {
            layer.weight = 0.0;
        }

        // Need at least 3 motions for 2D blending
        if self.motions.len() < 3 {
            if let Some(first) = self.blend_job.layers_mut().first_mut() {
                first.weight = 1.0;
            }
            return;
        }

        // Get all motion thresholds as Vec2
        let positions: Vec<Vec2> = self
            .motions
            .iter()
            .filter_map(|motion| {
                if let MotionThreshold::Directional2D(x, y) = motion.threshold {
                    Some(Vec2::new(x, y))
                } else {
                    None
                }
            })
            .collect();

        // Find the triangle that contains our point using barycentric coordinates
        let point = Vec2::new(x_param_value, y_param_value);
        for i in 0..positions.len() {
            let p1 = positions[i];

            for j in i + 1..positions.len() {
                let p2 = positions[j];

                for k in j + 1..positions.len() {
                    let p3 = positions[k];
                    // Calculate barycentric coordinates
                    let denominator = (p2.y - p3.y) * (p1.x - p3.x) + (p3.x - p2.x) * (p1.y - p3.y);
                    if denominator.abs() < f32::EPSILON {
                        continue;
                    }

                    let w1 = ((p2.y - p3.y) * (point.x - p3.x) + (p3.x - p2.x) * (point.y - p3.y))
                        / denominator;
                    let w2 = ((p3.y - p1.y) * (point.x - p3.x) + (p1.x - p3.x) * (point.y - p3.y))
                        / denominator;
                    let w3 = 1.0 - w1 - w2;

                    // If point is inside this triangle (all weights are positive)
                    if w1 >= 0.0 && w2 >= 0.0 && w3 >= 0.0 {
                        let layers = self.blend_job.layers_mut();
                        layers[i].weight = w1;
                        layers[j].weight = w2;
                        layers[k].weight = w3;
                        return;
                    }
                }
            }
        }

        // If point is outside all triangles, find nearest motion
        let mut nearest_idx = 0;
        let mut min_distance = f32::MAX;

        for (idx, pos) in positions.iter().enumerate() {
            let distance = point.distance(*pos);
            if distance < min_distance {
                min_distance = distance;
                nearest_idx = idx;
            }
        }

        // Set weight to 1.0 for nearest layer
        if let Some(layer) = self.blend_job.layers_mut().get_mut(nearest_idx) {
            layer.weight = 1.0;
        }
    }

    #[inline(always)]
    pub fn get_output_pointer(&self) -> Arc<RwLock<Vec<SoaTransform>>> {
        self.output.clone()
    }
}
