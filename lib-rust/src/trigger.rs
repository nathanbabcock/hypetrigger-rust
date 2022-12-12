use std::sync::Arc;

use crate::{any::AsAny, runner::RunnerFn};

pub type Triggers = Vec<Arc<dyn Trigger>>;

/// A **Trigger** defines a specific method of interpreting a video frame.
///
/// Concrete implementations do the following:
/// - specify a cropped rectangle of the frame to analyze
/// - run custom code to interpret the image data (the associated `runner` function)
pub trait Trigger: AsAny + Send + Sync {
    /// what region of the video to detect events in
    fn get_crop(&self) -> Crop;

    /// a unique identifier for this trigger *instance*
    /// Note: this is different than `Trigger::get_runner_type`
    fn get_id(&self) -> String;

    /// Enable debugging for this trigger, printing extra logs and saving
    /// intermediate snapshots to disk
    fn get_debug(&self) -> bool;

    /// Handles running triggers of this type, invoked in a separate thread.
    /// Note: returns a function pointer
    fn runner(&self) -> RunnerFn;

    /// a unique identifier used to map it to the correct Runner thread
    fn get_runner_type(&self) -> String;
}

/// Defines a rectangle within a image/video frame
/// - Units are pixels
/// - Origin is top-left (0, 0)
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,

    pub xPercent: f64,
    pub yPercent: f64,
    pub widthPercent: f64,
    pub heightPercent: f64,
}
