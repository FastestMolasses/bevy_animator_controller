use super::{AnimationState, Parameters};
use bevy::prelude::Time;
use ozz_animation_rs::{
    BlendingJob, BlendingJobArc, BlendingLayer, OzzError, Skeleton, SoaTransform,
};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

/// Represents a layer in the animator
#[derive(Debug)]
pub struct AnimationLayer {
    pub name: String,
    pub layer_blend_type: LayerBlendType,
    pub layer_weight: f32,
    default_state_name: String,
    states: HashMap<String, AnimationState>,
    transitions: HashMap<String, Vec<Transition>>,
    current_state: String,
    next_state: Option<String>,
    transition_time: f32,
    transition_duration: f32,
    pub is_transitioning: bool,
    blending_job: BlendingJobArc,
    blend_job_output: Arc<RwLock<Vec<SoaTransform>>>,
    /// If the source of the output has changed
    output_source_changed: bool,
}

impl AnimationLayer {
    #[inline]
    pub fn new(
        name: String,
        layer_blend_type: LayerBlendType,
        layer_weight: f32,
        skeleton: &Arc<Skeleton>,
        default_state_name: String,
    ) -> Self {
        let blend_job_output = Arc::new(RwLock::new(vec![
            SoaTransform::default();
            skeleton.num_soa_joints()
        ]));
        let mut blending_job: BlendingJobArc = BlendingJob::default();
        blending_job.set_skeleton(skeleton.clone());
        blending_job.set_output(blend_job_output.clone());

        Self {
            name,
            layer_weight,
            layer_blend_type,
            current_state: default_state_name.to_string(),
            default_state_name,
            states: HashMap::new(),
            transitions: HashMap::new(),
            next_state: None,
            transition_time: 0.0,
            transition_duration: 0.0,
            is_transitioning: false,
            blending_job,
            blend_job_output,
            // Default to true to force an update on the first frame
            output_source_changed: true,
        }
    }

    #[inline]
    pub fn add_state(&mut self, name: String, state: AnimationState) {
        self.states.insert(name, state);
    }

    #[inline]
    pub fn add_transition(&mut self, from_state: String, transition: Transition) {
        self.transitions
            .entry(from_state)
            .or_default()
            .push(transition);
    }

    #[inline]
    pub fn set_weight(&mut self, weight: f32) {
        self.layer_weight = weight.clamp(0.0, 1.0);
    }

    #[inline]
    fn check_transitions(&mut self, parameters: &Parameters) -> bool {
        let Some(transitions) = self.transitions.get(&self.current_state) else {
            return false;
        };
        if self.is_transitioning {
            return false;
        }
        for transition in transitions {
            let next_state = &transition.to_state;
            if self.evaluate_transition(transition, parameters) {
                // Make sure the next state exists
                if !self.states.contains_key(next_state) {
                    return false;
                }

                self.next_state = Some(next_state.to_string());
                self.transition_time = 0.0;
                self.transition_duration = transition.duration;
                self.is_transitioning = true;

                println!(
                    "Transitioning from {} to {}",
                    self.current_state, transition.to_state
                );
                return true;
            }
        }
        false
    }

    #[inline]
    fn evaluate_transition(&self, transition: &Transition, parameters: &Parameters) -> bool {
        if transition.has_exit_time {
            // TODO: Check exit time logic here
            // ...
        }

        // Validate all conditions
        for condition in &transition.conditions {
            if !self.evaluate_condition(condition, parameters) {
                return false;
            }
        }
        true
    }

    #[inline]
    fn evaluate_condition(&self, condition: &TransitionCondition, parameters: &Parameters) -> bool {
        match condition {
            TransitionCondition::Bool(name, value) => parameters.get_bool(name) == Some(*value),
            TransitionCondition::Float(name, value, compare_type) => {
                if let Some(param_value) = parameters.get_float(name) {
                    match compare_type {
                        CompareType::Greater => param_value > *value,
                        CompareType::Less => param_value < *value,
                        CompareType::Equals => (param_value - *value).abs() < f32::EPSILON,
                        CompareType::NotEqual => (param_value - *value).abs() >= f32::EPSILON,
                    }
                } else {
                    false
                }
            }
            TransitionCondition::Int(name, value, compare_type) => {
                if let Some(param_value) = parameters.get_int(name) {
                    match compare_type {
                        CompareType::Greater => param_value > *value,
                        CompareType::Less => param_value < *value,
                        CompareType::Equals => param_value == *value,
                        CompareType::NotEqual => param_value != *value,
                    }
                } else {
                    false
                }
            }
            TransitionCondition::Trigger(name) => parameters.get_trigger(name),
        }
    }

    #[inline]
    pub fn update(&mut self, time: &Time, parameters: &mut Parameters) -> Result<(), OzzError> {
        let was_transitioning = self.is_transitioning;

        self.check_transitions(parameters);

        // Detect transition state changes
        if was_transitioning != self.is_transitioning {
            self.output_source_changed = true;
        }

        // Update current state
        if let Some(current_state) = self.states.get_mut(&self.current_state) {
            match current_state {
                AnimationState::Simple(s) => {
                    s.update(time)?;
                }
                AnimationState::Blend(b) => {
                    b.update(time, parameters)?;
                }
            }
        }

        // Handle transition
        if let Some(next_state_name) = &self.next_state {
            self.transition_time += time.delta_secs();

            if self.transition_time >= self.transition_duration {
                // Transition complete
                self.current_state = next_state_name.clone();
                self.next_state = None;
                self.is_transitioning = false;
                self.output_source_changed = true;
            } else {
                // Blend between states
                let t = self.transition_time / self.transition_duration;

                // TODO: NEED TO CACHE POINTERS AND DONT RECONSTRUCT BLENDING LAYERS, JUST UPDATE THEM
                let current_state_output = self.states.get(&self.current_state).map(|s| match s {
                    AnimationState::Simple(state) => state.get_output_pointer(),
                    AnimationState::Blend(state) => state.get_output_pointer(),
                });
                let next_state_output = self.states.get_mut(next_state_name).map(|s| {
                    // We need to update the next state to get the output
                    match s {
                        AnimationState::Simple(state) => {
                            let _ = state.update(time);
                            state.get_output_pointer()
                        }
                        AnimationState::Blend(state) => {
                            let _ = state.update(time, parameters);
                            state.get_output_pointer()
                        }
                    }
                });

                if let (Some(current_output), Some(next_output)) =
                    (current_state_output, next_state_output)
                {
                    self.blend_states(current_output, next_output, t)?;
                }
            }
        }

        Ok(())
    }

    #[inline]
    fn blend_states(
        &mut self,
        current: Arc<RwLock<Vec<SoaTransform>>>,
        next: Arc<RwLock<Vec<SoaTransform>>>,
        t: f32,
    ) -> Result<(), OzzError> {
        self.blending_job.layers_mut().clear();

        // Construct blending layers
        self.blending_job.layers_mut().push(BlendingLayer {
            transform: current,
            weight: 1.0 - t,
            joint_weights: vec![],
        });
        self.blending_job.layers_mut().push(BlendingLayer {
            transform: next,
            weight: t,
            joint_weights: vec![],
        });

        self.blending_job.run()?;
        Ok(())
    }

    pub fn has_output_changed(&self) -> bool {
        self.output_source_changed
    }

    pub fn clear_output_changed(&mut self) {
        self.output_source_changed = false;
    }

    #[inline]
    pub(crate) fn get_output_pointer(&self) -> Arc<RwLock<Vec<SoaTransform>>> {
        if self.is_transitioning {
            self.blend_job_output.clone()
        } else {
            // TODO: WE NEED TO BE IN "T POSE" or DEFAULT POSE IF NO STATE IS FOUND
            self.states
                .get(&self.current_state)
                .map(|s| match s {
                    AnimationState::Simple(state) => state.get_output_pointer(),
                    AnimationState::Blend(state) => state.get_output_pointer(),
                })
                .unwrap_or(self.blend_job_output.clone())
        }
    }
}

/// The type of blending to use for a layer
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayerBlendType {
    Override,
    Additive,
}

/// Represents a transition to a state
#[derive(Debug)]
pub struct Transition {
    pub to_state: String,
    pub duration: f32,
    pub conditions: Vec<TransitionCondition>,
    pub has_exit_time: bool,
    pub exit_time: f32,
}

/// Condition for state transitions
#[derive(Debug, Clone)]
pub enum TransitionCondition {
    Bool(String, bool),
    Float(String, f32, CompareType),
    Int(String, i32, CompareType),
    Trigger(String),
}

#[derive(Debug, Clone)]
pub enum CompareType {
    Greater,
    Less,
    Equals,
    NotEqual,
}
