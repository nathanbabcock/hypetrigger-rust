#[macro_use]
extern crate derive_builder;

#[macro_use]
extern crate lazy_static;

pub mod any;
pub mod config;
pub mod debugger;
pub mod emit;
pub mod ffmpeg;
pub mod logging;
pub mod main_thread;
pub mod photon;
pub mod pipeline;
pub mod pipeline_simple;
pub mod runner;
pub mod threshold;
pub mod trigger;

#[cfg(feature = "tensorflow")]
pub mod tensorflow;

#[cfg(feature = "tesseract")]
pub mod tesseract;
