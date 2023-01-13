#[cfg(not(feature = "wasm"))]
#[macro_use]
extern crate lazy_static;

//// Image processing modules
// Required for tesseract/tensorflow, but can be skipped for simple or custom
// triggers. Also required for wasm builds and provides a rich image library.
#[cfg(feature = "photon")]
pub mod photon;

#[cfg(feature = "photon")]
pub mod threshold;

//// Specific trigger implementations (tesseract, tensorflow)
#[cfg(feature = "tesseract")]
pub mod tesseract;

#[cfg(feature = "tensorflow")]
pub mod tensorflow;

//// Tests
#[cfg(test)]
mod tests;

//// Core functionality
// Not WASM-safe; intended for Rust usage only. Involves spawning and attaching to ffmpeg processes.
#[cfg(not(feature = "wasm"))]
pub mod async_trigger;

#[cfg(not(feature = "wasm"))]
pub mod debug;

#[cfg(not(feature = "wasm"))]
pub mod error;

#[cfg(not(feature = "wasm"))]
pub mod pipeline;

#[cfg(not(feature = "wasm"))]
pub mod simple_trigger;

#[cfg(not(feature = "wasm"))]
pub mod trigger;

#[cfg(not(feature = "wasm"))]
pub mod util;
