use std::sync::Arc;

use crate::{any::AsAny, runner::RunnerFn};

pub type Triggers = Vec<Arc<Trigger>>;

/// A **Trigger** defines a specific method of interpreting a video frame.
///
/// Concrete implementations do the following:
/// - specify a cropped rectangle of the frame to analyze
/// - run custom code to interpret the image data (the associated `runner` function)
pub struct Trigger {
    pub id: String,
    pub crop: Crop,
    pub debug: bool,

    /// Additional parameters that are speficic to this variety of Trigger
    /// - For **Tesseract**, it would be the threshold filter settings
    /// - For **Tensorflow**, it would be the model directory
    /// - For **custom Triggers**, it is a user-defined struct that extends `TriggerParams`
    pub params: Arc<dyn TriggerParams>,
}

/// Parameters to defined specific behavior for a type of Trigger
pub trait TriggerParams: AsAny + Send + Sync {
    /// The key used in the HashMap of Runners, used to match it with this Trigger
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
