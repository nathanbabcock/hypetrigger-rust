use crate::error::Result;
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

    // /// Convert this Trigger into a ThreadTrigger, running on a separate thread.
    // fn into_thread(self, runner_thread: Arc<RunnerThread>) -> ThreadTrigger
    // where
    //     Self: Sized + Send + Sync + 'static,
    // {
    //     ThreadTrigger {
    //         trigger: Arc::new(self),
    //         runner_thread,
    //     }
    // }
}
