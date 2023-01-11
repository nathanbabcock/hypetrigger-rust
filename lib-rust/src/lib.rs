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

pub mod any;
pub mod async_trigger;
pub mod config;
pub mod debug;
pub mod debugger;
pub mod emit;
pub mod error;
pub mod ffmpeg;
pub mod logging;
pub mod main_thread;
pub mod pipeline;
pub mod pipeline_simple;
pub mod runner;
pub mod simple_trigger;
pub mod trigger;
pub mod util;
