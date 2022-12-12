use std::cmp::min;

use photon_rs::{
    transform::{crop, resize, SamplingFilter},
    PhotonImage,
};
use wasm_bindgen::prelude::wasm_bindgen;

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
