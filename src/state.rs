use super::blend_tree::BlendTree;
use super::Parameters;
use ozz_animation_rs::{Animation, SamplingContext, SamplingJob, SamplingJobArc, SoaTransform, OzzError};
use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use bevy::prelude::Time;

// TODO: TIME SHOULDNT BE USING ELAPSED_SECS, WE SHOULD BE ABLE TO CONTROL IT
/// Base trait for animation states
// pub trait AnimationState: Send + Sync + Debug {
//     fn update(&mut self, time: &Time) -> Result<(), OzzError>;
//     fn get_output_pointer(&self) -> Arc<RwLock<Vec<SoaTransform>>>;
//     fn get_duration(&self) -> f32;
// }

#[derive(Debug)]
pub enum AnimationState {
    Simple(SimpleState),
    Blend(BlendState),
}

/// Simple state containing a single animation
#[derive(Debug)]
pub struct SimpleState {
    sampling_job: SamplingJobArc,
    output: Arc<RwLock<Vec<SoaTransform>>>,
    duration: f32,
}

unsafe impl Send for SimpleState {}
unsafe impl Sync for SimpleState {}

impl SimpleState {
    /// Create a new simple state that holds a single animation.
    /// ## Example
    /// ```
    /// use bevy::tasks::futures_lite::future::try_zip;
    ///
    /// let (mut skeleton, mut animation) = try_zip(
    ///     load_archive("/skeleton.ozz"),
    ///     load_archive("/idle_animation.ozz"),
    /// )
    /// .await
    /// .unwrap();
    ///
    /// let skeleton = Arc::new(Skeleton::from_archive(&mut skeleton).unwrap());
    /// let animation = Arc::new(Animation::from_archive(&mut animation).unwrap());
    /// let state = SimpleState::new(animation, skeleton.num_soa_joints());
    /// ```
    #[inline]
    pub fn new(animation: Arc<Animation>, joint_count: usize) -> Self {
        let mut sampling_job: SamplingJobArc = SamplingJob::default();
        sampling_job.set_context(SamplingContext::new(animation.num_tracks()));
        sampling_job.set_animation(animation.clone());
        let sample_out = Arc::new(RwLock::new(vec![SoaTransform::default(); joint_count]));
        sampling_job.set_output(sample_out.clone());

        Self {
            sampling_job,
            output: sample_out,
            duration: 0.0,
        }
    }
}

impl SimpleState {
    #[inline]
    pub fn update(&mut self, time: &Time) -> Result<(), OzzError> {
        let Some(animation) = self.sampling_job.animation() else {
            return Ok(());
        };
        let duration = animation.duration();
        self.sampling_job.set_ratio((time.elapsed_secs() % duration) / duration);
        self.sampling_job.run()?;
        Ok(())
    }

    #[inline]
    pub fn get_output_pointer(&self) -> Arc<RwLock<Vec<SoaTransform>>> {
        self.output.clone()
    }

    #[inline]
    fn get_duration(&self) -> f32 {
        self.duration
    }
}

/// State containing a blend tree
#[derive(Debug)]
pub struct BlendState {
    blend_tree: BlendTree,
    duration: f32,
}

unsafe impl Send for BlendState {}
unsafe impl Sync for BlendState {}

impl BlendState {
    #[inline]
    pub fn new(blend_tree: BlendTree) -> Self {
        Self {
            blend_tree,
            duration: 0.0,
        }
    }
}

impl BlendState {
    #[inline]
    pub fn update(&mut self, time: &Time, params: &mut Parameters) -> Result<(), OzzError> {
        self.blend_tree.update(time, params)?;
        Ok(())
    }

    #[inline]
    pub fn get_output_pointer(&self) -> Arc<RwLock<Vec<SoaTransform>>> {
        self.blend_tree.get_output_pointer()
    }

    #[inline]
    fn get_duration(&self) -> f32 {
        self.duration
    }
}
