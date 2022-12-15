use std::sync::Arc;

use crate::{any::AsAny, runner::RunnerFn};

pub type Triggers = Vec<Arc<dyn Trigger>>;

/// Try one last time to regain sanity
/// This is a Trait because different varieties require different parameters
pub trait Trigger: AsAny + Send + Sync {
    /// A string key used in the HashMap of Runners, used to match it with
    /// instances of this particular implementation of Trigger.
    /// Takes `&self` so that it can be [object-safe](https://doc.rust-lang.org/reference/items/traits.html#object-safety)
    fn get_runner_type(&self) -> String;

    /// TODO I'm almost sold on removing this, and making it the responsibility
    /// of the RunnerFn. It stays for now to avoid changing too much at once.
    fn get_crop(&self) -> Crop;

    // TODO an ID is probably still needed, for debugging and logging purposes.
    // For now though, it's been delegated to individual implementations.
    // ~~Tensorflow needs it to map triggers to their models.~~
    // (Actually on second thought, it could trivially use the model path as
    // the hashmap key -- and it probably should.)
    //
    // The API design philosophy here is to 100% let Trigger implementations
    // decide what to standardize around.
    //
    // - Do they all have a `debug` option?
    // - Do they all `crop` at the beginning?
    // - Do they share image preprocessing logic?
    //
    // All of these need to be *possible*, but the library doesn't need to
    // enforce them. For example, the Hypetrigger app will make some of these
    // decisions, and encode a JSON format that reflects it, but the library
    // implementation can remain agnostic.
    //
    // A sub-trait of Trigger is even possible when that enforcement is desired
    // -- so nothing is lost by not enforcing it here!
    //
    // fn get_id(&self) -> String;
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
