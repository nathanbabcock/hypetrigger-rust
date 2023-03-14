use crate::{
    async_trigger::{AsyncTrigger, TriggerCommand},
    error::Result,
};
use image::RgbImage;
use std::sync::mpsc::SyncSender;

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
    fn into_async(self, runner_tx: SyncSender<TriggerCommand>) -> AsyncTrigger
    where
        Self: Sized + 'static,
    {
        AsyncTrigger::from_trigger(self, runner_tx)
    }
}
