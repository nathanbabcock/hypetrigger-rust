use std::sync::Arc;

use hypetrigger::error::Result;
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
        crop: None,
        threshold_filter: None,
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

    println!("done");
    Ok(())
}
