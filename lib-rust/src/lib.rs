#[macro_use]
extern crate lazy_static;

#[cfg(feature = "photon")]
pub mod photon;

#[cfg(feature = "photon")]
pub mod threshold;

#[cfg(feature = "tensorflow")]
pub mod tensorflow;

#[cfg(feature = "tesseract")]
pub mod tesseract;

#[cfg(test)]
mod tests;

pub mod async_trigger;
pub mod debug;
pub mod error;
pub mod logging;
pub mod pipeline;
pub mod simple_trigger;
pub mod trigger;
pub mod util;
