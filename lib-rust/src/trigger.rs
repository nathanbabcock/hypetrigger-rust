use std::sync::Arc;

use crate::{
    async_trigger::{AsyncTrigger, TriggerThread},
    error::Result,
};
use image::RgbImage;

/// Represents a single frame of the input, including the raw image pixels as
/// well as the time it appears in the input (frame_num and/or timestamp)
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub image: RgbImage,
    pub frame_num: u64,
    pub timestamp: f64,
}

//// Triggers
pub trait Trigger: Send + Sync {
    fn on_frame(&self, frame: &Frame) -> Result<()>;

    /// Convert this Trigger into a `AsyncTrigger`, running on a separate thread.
    fn into_async(self, runner_thread: Arc<TriggerThread>) -> AsyncTrigger
    where
        Self: Sized + 'static,
    {
        AsyncTrigger::from_trigger(self, runner_thread)
    }
}
