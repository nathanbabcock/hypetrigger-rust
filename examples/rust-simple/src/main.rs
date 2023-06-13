use std::{
    io::{BufReader, Read},
    process::{Command, Stdio},
};

fn main() {
    // Test video provided by https://gist.github.com/jsturgis/3b19447b304616f18657
    let test_video =
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4";

    // Video is in RGB format; 3 bytes per pixel (one red, one blue, one green)
    let bytes_per_pixel = 3;

    // Video is 1280x720 resolution
    let video_width = 1280;
    let video_height = 720;

    // Create an FFmpeg command with the specified arguments
    let mut ffmpeg = Command::new("ffmpeg") // request 1 frame per second sampled from original
        .arg("-i")
        .arg(test_video) // specify the input video
        .arg("-f") // specify the output format (raw RGB pixels)
        .arg("rawvideo")
        .arg("-pix_fmt")
        .arg("rgb24") // specify the pixel format (RGB, 8 bits per channel)
        .arg("-r")
        .arg("1") // Request rate of 1 frame per second
        .arg("pipe:1") // send output to the stdout pipe
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn() // spawn the command process
        .unwrap(); // unwrap the result (i.e. panic and exit if there was an error)

    // Read the video output into a buffer
    let stdout = ffmpeg.stdout.take().unwrap();
    let buf_size = video_width * video_height * bytes_per_pixel;
    let mut reader = BufReader::new(stdout);
    let mut buffer = vec![0u8; buf_size];
    let mut frame_num = 0;

    while let Ok(()) = reader.read_exact(buffer.as_mut_slice()) {
        // Retrieve each video frame as a vector of raw RGB pixels
        let raw_rgb = buffer.clone();

        // Calculate the average brightness of the frame
        let brightness = average_brightness(raw_rgb);
        println!("frame {frame_num} has brightness {brightness}");
        frame_num += 1;
    }
}

/// Calculate the average brightness of an image,
/// returned as a float between 0 and 1.
fn average_brightness(raw_rgb: Vec<u8>) -> f64 {
    let mut sum = 0.0;
    for (i, _) in raw_rgb.iter().enumerate().step_by(3) {
        let r = raw_rgb[i] as f64;
        let g = raw_rgb[i + 1] as f64;
        let b = raw_rgb[i + 2] as f64;
        let pixel_brightness = (r / 255.0 + g / 255.0 + b / 255.0) / 3.0;
        sum += pixel_brightness;
    }
    sum / (raw_rgb.len() as f64 / 3.0)
}
