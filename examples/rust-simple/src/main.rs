use hypetrigger::{error::Result, pipeline::Hypetrigger, simple_trigger::SimpleTrigger};
use image::RgbImage;

/// Calculate the average brightness of an image,
/// returned as a float between 0 and 1.
fn average_brightness(image: &RgbImage) -> f64 {
    let mut sum = 0.0;
    let width = image.width() as f64;
    let height = image.height() as f64;
    for pixel in image.pixels() {
        let r = pixel[0] as f64;
        let g = pixel[1] as f64;
        let b = pixel[2] as f64;
        sum += (r + g + b) / (255.0 * 3.0);
    }
    sum / (width * height)
}

fn main() -> Result<()> {
    println!("Hello, world!");

    // Test video provided by https://gist.github.com/jsturgis/3b19447b304616f18657
    let test_video =
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
            .to_string();

    Hypetrigger::new()
        .set_input(test_video)
        .set_fps(1)
        .add_trigger(SimpleTrigger::new(|frame| {
            let brightness = average_brightness(&frame.image);
            println!("frame {} has brightness {}", frame.frame_num, brightness);
        }))
        .run()
}
