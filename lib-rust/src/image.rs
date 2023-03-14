use ffmpeg_sidecar::event::OutputVideoFrame;
use photon_rs::PhotonImage;

use crate::photon::rgb24_to_rgba32;

/// An image with chainable methods for image processing. Compatible with both
/// WASM and native Rust.
pub struct HypetriggerImage {
  photon_image: PhotonImage,
}

impl HypetriggerImage {
  pub fn get_width(&self) -> u32 {
    self.photon_image.get_width()
  }

  pub fn get_height(&self) -> u32 {
    self.photon_image.get_height()
  }

  /// Borrow the inner `PhotonImage`.
  pub fn as_photon_image(&self) -> &PhotonImage {
    &self.photon_image
  }

  /// Consume a `HypetriggerImage` and return the underlying `PhotonImage`.
  pub fn into_photon_image(self) -> PhotonImage {
    self.photon_image
  }
}

impl From<PhotonImage> for HypetriggerImage {
  fn from(photon_image: PhotonImage) -> Self {
    Self { photon_image }
  }
}

impl From<OutputVideoFrame> for HypetriggerImage {
  fn from(frame: OutputVideoFrame) -> Self {
    let rgb32 = match frame.pix_fmt.as_str() {
      "rgb32" => frame.data,
      "rgb24" => rgb24_to_rgba32(frame.data),
      _ => panic!("Unsupported pixel format"),
    };
    let photon_image = PhotonImage::new(rgb32, frame.width, frame.height);
    Self { photon_image }
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn test_from_frame() {
    panic!();
  }
}
