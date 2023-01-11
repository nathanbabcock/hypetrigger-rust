use crate::error::Result;
use crate::trigger::{Frame, Trigger};
use std::sync::Arc;

/// A minimal Trigger implementation that just calls a callback on each frame.
/// Functionally equivalent to a custom struct that implements `Trigger`, just
/// with a callback instead of the `on_frame` trait method.
#[derive(Clone)]
pub struct SimpleTrigger {
    pub callback: Arc<dyn Fn(&Frame) + Send + Sync>,
}

impl Trigger for SimpleTrigger {
    fn on_frame(&self, frame: &Frame) -> Result<()> {
        (self.callback)(frame);
        Ok(())
    }
}

impl SimpleTrigger {
    pub fn new<T>(on_frame: T) -> Self
    where
        T: Fn(&Frame) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(on_frame),
        }
    }
}
