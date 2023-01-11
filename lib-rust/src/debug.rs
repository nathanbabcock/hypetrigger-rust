use crate::error::NoneError;
use crate::trigger::Frame;
use crate::{error::Result, util::format_seconds};
use image::{DynamicImage, RgbImage};
use std::env::current_exe;
use std::io::stdin;

/// Write image to disk and pause execution.
pub fn debug_image(image: &DynamicImage) -> Result<()> {
    let preview_path = current_exe()?
        .parent()
        .ok_or(NoneError)?
        .join("debug-image.bmp");
    image.save(&preview_path)?;

    println!("[debug] Preview image saved to {}", &preview_path.display());
    println!("[debug] Press any key to continue...");
    stdin().read_line(&mut String::new())?;
    Ok(())
}

/// Write current frame to disk and pause execution.
pub fn debug_frame(frame: &Frame) -> Result<()> {
    println!(
        "[debug] Execution paused on frame {} ({})",
        frame.frame_num,
        format_seconds(frame.timestamp)
    );
    debug_rgb(&frame.image)
}

/// Write image to disk and pause execution.
pub fn debug_rgb(image: &RgbImage) -> Result<()> {
    debug_image(&DynamicImage::ImageRgb8(image.clone()))
}

/// Write image to disk and pause execution.
#[cfg(feature = "photon")]
pub fn debug_photon_image(image: &photon_rs::PhotonImage) -> Result<()> {
    let dynamic_image = photon_rs::helpers::dyn_image_from_raw(image);
    debug_image(&dynamic_image)
}
