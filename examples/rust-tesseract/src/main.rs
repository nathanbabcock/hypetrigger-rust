use std::sync::Arc;

use hypetrigger::error::Result;
use hypetrigger::photon::{Crop, ThresholdFilter};
use hypetrigger::pipeline::Hypetrigger;
use hypetrigger::tesseract::{init_tesseract, TesseractTrigger};

fn main() -> Result<()> {
    println!("Hello, world!");

    // Initialize tesseract, the OCR library used for text recognition
    // Using all default settings (english alphabet & language)
    let tesseract = init_tesseract(None, None)?;

    // Create a trigger to detect the counter
    let trigger = TesseractTrigger {
        tesseract,
        crop: Some(Crop {
            top_percent: 256.0 * 100.0 / 720.0,
            left_percent: 1024.0 * 100.0 / 1280.0,
            width_percent: 128.0 * 100.0 / 1280.0,
            height_percent: 208.0 * 100.0 / 720.0,
        }),
        threshold_filter: Some(ThresholdFilter {
            r: 255,
            g: 255,
            b: 255,
            threshold: 42,
        }),
        callback: Some(Arc::new(|test| {
            println!("{}", test.text); // print the recognized text
        })),
        enable_debug_breakpoints: false,
    };

    // Start the job
    Hypetrigger::new()
        // Generate a test video with ffmpeg
        // It consists of a gradient color pattern
        // and a counter that increments every second
        .test_input()
        .set_fps(1)
        .add_trigger(trigger)
        .run()?;
    // TODO: doesn't recognize digital clock digits very easily

    println!("done");
    Ok(())
}
