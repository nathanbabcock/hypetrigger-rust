use photon_rs::{PhotonImage, Rgb};
use wasm_bindgen::prelude::wasm_bindgen;

pub type Lab = (f64, f64, f64);

/** https://en.wikipedia.org/wiki/CIELAB_color_space */
pub fn rgb2lab(rgb: &Rgb) -> Lab {
    let mut r = rgb.get_red() as f64 / 255_f64;
    let mut g = rgb.get_green() as f64 / 255_f64;
    let mut b = rgb.get_blue() as f64 / 255_f64;
    r = if r > 0.04045 {
        f64::powf((r + 0.055) / 1.055, 2.4)
    } else {
        r / 12.92
    };
    g = if g > 0.04045 {
        f64::powf((g + 0.055) / 1.055, 2.4)
    } else {
        g / 12.92
    };
    b = if b > 0.04045 {
        f64::powf((b + 0.055) / 1.055, 2.4)
    } else {
        b / 12.92
    };
    let mut x = (r * 0.4124 + g * 0.3576 + b * 0.1805) / 0.95047;
    let mut y = (r * 0.2126 + g * 0.7152 + b * 0.0722) / 1.00000;
    let mut z = (r * 0.0193 + g * 0.1192 + b * 0.9505) / 1.08883;
    x = if x > 0.008856 {
        f64::powf(x, 1.0 / 3.0)
    } else {
        (7.787 * x) + 16.0 / 116.0
    };
    y = if y > 0.008856 {
        f64::powf(y, 1.0 / 3.0)
    } else {
        (7.787 * y) + 16.0 / 116.0
    };
    z = if z > 0.008856 {
        f64::powf(z, 1.0 / 3.0)
    } else {
        (7.787 * z) + 16.0 / 116.0
    };
    ((116.0 * y) - 16.0, 500.0 * (x - y), 200.0 * (y - z))
}

#[wasm_bindgen]
/** https://en.wikipedia.org/wiki/Color_difference */
pub fn delta_e(color_a: &Rgb, color_b: &Rgb) -> f64 {
    let lab_a = rgb2lab(color_a);
    let lab_b = rgb2lab(color_b);
    let delta_l = lab_a.0 - lab_b.0;
    let delta_a = lab_a.1 - lab_b.1;
    let delta_b = lab_a.2 - lab_b.2;
    let c1 = (lab_a.1 * lab_a.1 + lab_a.2 * lab_a.2).sqrt();
    let c2 = (lab_b.1 * lab_b.1 + lab_b.2 * lab_b.2).sqrt();
    let delta_c = c1 - c2;
    let mut delta_h = delta_a * delta_a + delta_b * delta_b - delta_c * delta_c;
    delta_h = if delta_h < 0.0 { 0.0 } else { delta_h.sqrt() };
    let sc = 1.0 + 0.045 * c1;
    let sh = 1.0 + 0.015 * c1;
    let delta_lklsl = delta_l / 1.0;
    let delta_ckcsc = delta_c / sc;
    let delta_hkhsh = delta_h / sh;
    let i = delta_lklsl * delta_lklsl + delta_ckcsc * delta_ckcsc + delta_hkhsh * delta_hkhsh;
    if i < 0.0 { 0.0 } else { i.sqrt() }
}

#[wasm_bindgen]
/** Custom thresholding function which uses the color distance from a given color */
pub fn threshold_color_distance(image: PhotonImage, color: &Rgb, threshold: f64) -> PhotonImage {
    PhotonImage::new(
        threshold_color_distance_rgba(image.get_raw_pixels(), color, threshold),
        image.get_width(),
        image.get_height(),
    )
}

#[wasm_bindgen]
/** Custom thresholding function which uses the color distance from a given color */
pub fn threshold_color_distance_rgba(vector: Vec<u8>, color: &Rgb, threshold: f64) -> Vec<u8> {
    let mut new_vector = Vec::new();

    for i in (0..vector.len()).step_by(4) {
        let r = vector[i];
        let g = vector[i + 1];
        let b = vector[i + 2];

        let px_color: Rgb = Rgb::new(r, g, b);
        let v = if delta_e(&px_color, color) >= threshold {
            255u8
        } else {
            0u8
        };
        new_vector.push(v);
        new_vector.push(v);
        new_vector.push(v);
        new_vector.push(255u8); // alpha
    }

    new_vector
}
