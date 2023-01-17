use image::DynamicImage::ImageRgba8;
use image::GenericImageView;
use image::{ImageBuffer, RgbImage, RgbaImage};
use photon_rs::{
    helpers,
    transform::{resize, SamplingFilter},
    PhotonImage, Rgb,
};
use std::cmp::min;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::iter::ImageIterator;
use crate::threshold::threshold_color_distance_rgba;

/// A threshold function based on perceptual color distance
#[wasm_bindgen]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct ThresholdFilter {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub threshold: u8,
}

#[wasm_bindgen]
impl ThresholdFilter {
    pub fn apply(&self, image: PhotonImage) -> PhotonImage {
        let color = Rgb::new(self.r, self.g, self.b);
        let raw_pixels =
            threshold_color_distance_rgba(image.get_raw_pixels(), &color, self.threshold as f64);
        PhotonImage::new(raw_pixels, image.get_width(), image.get_height())
    }
}

#[wasm_bindgen]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Crop {
    pub left_percent: f64,
    pub top_percent: f64,
    pub width_percent: f64,
    pub height_percent: f64,
}

#[wasm_bindgen]
impl Crop {
    pub fn apply(&self, mut image: PhotonImage) -> PhotonImage {
        let width = image.get_width() as f64;
        let height = image.get_height() as f64;
        let x1 = (width * (self.left_percent / 100.0)) as u32;
        let y1 = (height * (self.top_percent / 100.0)) as u32;
        let x2 = (x1 as f64 + (self.width_percent * width / 100.0)) as u32;
        let y2 = (y1 as f64 + (self.height_percent * height / 100.0)) as u32;
        crop(&mut image, x1, y1, x2, y2)
    }
}

/// Fixed version of `crop` from `photon-rs@0.3.1`.
/// Fixed on `master` branch, but never published.
/// <https://github.com/silvia-odwyer/photon/pull/100>
pub fn crop(photon_image: &mut PhotonImage, x1: u32, y1: u32, x2: u32, y2: u32) -> PhotonImage {
    let img = helpers::dyn_image_from_raw(photon_image);
    let mut cropped_img: RgbaImage = ImageBuffer::new(x2 - x1, y2 - y1);

    for (x, y) in ImageIterator::with_dimension(&cropped_img.dimensions()) {
        let px = img.get_pixel(x1 + x, y1 + y);
        cropped_img.put_pixel(x, y, px);
    }
    let dynimage = ImageRgba8(cropped_img);
    let raw_pixels = dynimage.to_bytes();
    PhotonImage::new(raw_pixels, dynimage.width(), dynimage.height())
}

/// Resize if needed and reserve aspect ratio
#[wasm_bindgen]
pub fn ensure_minimum_size(image: &PhotonImage, min_size: u32) -> PhotonImage {
    let mut width = image.get_width();
    let mut height = image.get_height();

    if width < min_size {
        let scale = min_size as f32 / width as f32;
        width = (width as f32 * scale) as u32;
        height = (height as f32 * scale) as u32;
    }

    if height < min_size {
        let scale = min_size as f32 / height as f32;
        width = (width as f32 * scale) as u32;
        height = (height as f32 * scale) as u32;
    }

    if width != image.get_width() || height != image.get_height() {
        resize(image, width, height, SamplingFilter::Lanczos3)
    } else {
        image.clone()
    }
}

#[wasm_bindgen]
pub fn is_square(image: &PhotonImage) -> bool {
    image.get_width() == image.get_height()
}

#[wasm_bindgen]
pub fn ensure_square(image: PhotonImage) -> PhotonImage {
    if !is_square(&image) {
        center_square_crop(image)
    } else {
        image
    }
}

#[wasm_bindgen]
pub fn center_square_crop(image: PhotonImage) -> PhotonImage {
    let mut image = image;
    let side_length = min(image.get_width(), image.get_height());
    let x1 = (image.get_width() - side_length) / 2;
    let y1 = (image.get_height() - side_length) / 2;
    let x2 = x1 + side_length;
    let y2 = y1 + side_length;
    crop(&mut image, x1, y1, x2, y2)
}

/// Resize if needed, NOT preserving aspect ratio
#[wasm_bindgen]
pub fn ensure_size(image: PhotonImage, width: u32, height: u32) -> PhotonImage {
    let is_correct_size = image.get_width() == width && image.get_height() == height;
    if !is_correct_size {
        resize(&image, width, height, SamplingFilter::Lanczos3)
    } else {
        image
    }
}

#[wasm_bindgen]
pub fn rgb24_to_rgba32(vec: Vec<u8>) -> Vec<u8> {
    let mut new_vec = Vec::with_capacity(vec.len() * 4 / 3);
    for i in (0..vec.len()).step_by(3) {
        new_vec.push(vec[i]);
        new_vec.push(vec[i + 1]);
        new_vec.push(vec[i + 2]);
        new_vec.push(255);
    }
    new_vec
}

#[wasm_bindgen]
pub fn rgba32_to_rgb24(vec: Vec<u8>) -> Vec<u8> {
    let mut new_vec = Vec::with_capacity(vec.len() * 3 / 4);
    for i in (0..vec.len()).step_by(4) {
        new_vec.push(vec[i]);
        new_vec.push(vec[i + 1]);
        new_vec.push(vec[i + 2]);
    }
    new_vec
}

/// Convert an `RgbImage` (`image` crate) to a `PhotonImage` (`photon-rs` crate)
pub fn rgb_to_photon(rgb: &RgbImage) -> PhotonImage {
    let rgb24 = rgb.to_vec();
    let rgb32 = rgb24_to_rgba32(rgb24);

    PhotonImage::new(rgb32, rgb.width(), rgb.height())
}
