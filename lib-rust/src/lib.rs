#[cfg(not(target_arch = "wasm32"))]
#[macro_use]
extern crate lazy_static;

//// Image processing modules
// Required for tesseract/tensorflow, but can be skipped for simple or custom
// triggers. Also required for wasm builds and provides a rich image library.
#[cfg(feature = "photon")]
pub mod photon;

#[cfg(feature = "photon")]
pub mod iter;

#[cfg(feature = "photon")]
pub mod threshold;

//// Specific trigger implementations (tesseract, tensorflow)
#[cfg(feature = "tesseract")]
pub mod tesseract;

#[cfg(feature = "tensorflow")]
pub mod tensorflow;

//// Core functionality
// Not WASM-safe; intended for Rust usage only. Involves spawning and attaching to ffmpeg processes.
#[cfg(not(target_arch = "wasm32"))]
pub mod async_trigger;

#[cfg(not(target_arch = "wasm32"))]
pub mod debug;

#[cfg(not(target_arch = "wasm32"))]
pub mod error;

#[cfg(not(target_arch = "wasm32"))]
pub mod pipeline;

#[cfg(not(target_arch = "wasm32"))]
pub mod simple_trigger;

#[cfg(not(target_arch = "wasm32"))]
pub mod trigger;

#[cfg(not(target_arch = "wasm32"))]
pub mod util;
