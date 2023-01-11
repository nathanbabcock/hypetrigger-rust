#[macro_use]
extern crate lazy_static;

pub mod any;
pub mod config;
pub mod debugger;
pub mod emit;
pub mod ffmpeg;
pub mod logging;
pub mod main_thread;
pub mod pipeline;
pub mod pipeline_simple;
pub mod runner;
pub mod trigger;

#[cfg(feature = "photon")]
pub mod photon;
pub mod threshold;

#[cfg(feature = "tensorflow")]
pub mod tensorflow;

pub mod debug;
pub mod error;
#[cfg(feature = "tesseract")]
pub mod tesseract;
