use crate::debug::debug_photon_image;
use crate::error::{NoneError, Result};
use crate::photon::{ensure_minimum_size, rgb_to_photon, Crop, ThresholdFilter};
use crate::trigger::{Frame, Trigger};
use photon_rs::transform::padding_uniform;
use photon_rs::{PhotonImage, Rgba};
use std::io::Write;
use std::sync::Arc;
use std::{
    fs::{self, File},
    path::Path,
    sync::Mutex,
};
use tesseract::Tesseract;

pub type TesseractTriggerCallback = Arc<dyn Fn(TesseractResult) + Send + Sync>;

#[derive(Clone, Debug, PartialEq)]
pub struct TesseractResult {
    pub text: String,
    pub timestamp: f64,
    pub frame_num: u64,
}

#[derive(Clone)]
pub struct TesseractTrigger {
    /// The initialized instance of Tesseract that will be used to run this trigger
    pub tesseract: TesseractRef,

    /// The region to crop to before running OCR.
    pub crop: Option<Crop>,

    /// The threshold filter to apply before running OCR.
    pub threshold_filter: Option<ThresholdFilter>,

    /// The callback to run after OCR is complete.
    pub callback: Option<TesseractTriggerCallback>,

    /// Pause execution after each step of image pre-processing.
    pub enable_debug_breakpoints: bool,
}

impl Trigger for TesseractTrigger {
    fn on_frame(&self, frame: &Frame) -> Result<()> {
        // 1. convert raw image to photon
        let image = rgb_to_photon(&frame.image);

        // 2. preprocess
        let filtered = self.preprocess_image(image)?;

        // 3. run ocr
        let text = self.ocr(filtered)?;

        // 4. callback
        if let Some(callback) = &self.callback {
            let result = TesseractResult {
                text,
                timestamp: frame.timestamp,
                frame_num: frame.frame_num,
            };
            callback(result);
        }

        Ok(())
    }
}

impl TesseractTrigger {
    pub fn new() -> Self {
        Self {
            tesseract: Arc::new(Mutex::new(None)),
            crop: None,
            threshold_filter: None,
            callback: None,
            enable_debug_breakpoints: false,
        }
    }

    pub fn preprocess_image(&self, mut image: PhotonImage) -> Result<PhotonImage> {
        if self.enable_debug_breakpoints {
            println!("[tesseract] received frame");
            debug_photon_image(&image)?;
        }

        // Crop
        if let Some(crop) = &self.crop {
            image = crop.apply(image);
            if self.enable_debug_breakpoints {
                println!("[tesseract] crop: {:?}", self.crop);
                debug_photon_image(&image)?;
            }
        }

        // Minimum size
        const MIN_TESSERACT_IMAGE_SIZE: u32 = 32;
        image = ensure_minimum_size(&image, MIN_TESSERACT_IMAGE_SIZE);
        if self.enable_debug_breakpoints {
            println!("[tesseract] resized");
            debug_photon_image(&image)?;
        }

        // Threshold filter
        if let Some(filter) = &self.threshold_filter {
            image = filter.apply(image);
            if self.enable_debug_breakpoints {
                println!("[tesseract] filter: {:?}", filter);
                debug_photon_image(&image)?;
            }
        }

        // Padding
        let padding_bg: Rgba = Rgba::new(255, 255, 255, 255);
        image = padding_uniform(&image, MIN_TESSERACT_IMAGE_SIZE, padding_bg);
        if self.enable_debug_breakpoints {
            println!("[tesseract] padded (done)");
            debug_photon_image(&image)?;
        }

        Ok(image)
    }

    pub fn ocr(&self, image: PhotonImage) -> Result<String> {
        let rgba32 = image.get_raw_pixels();
        let buf = rgba32.as_slice();
        let channels = 4;

        let mut mutex_guard = self.tesseract.lock()?;
        let mut tesseract = mutex_guard.take().ok_or(NoneError)?;
        tesseract = tesseract
            .set_frame(
                buf,
                image.get_width() as i32,
                image.get_height() as i32,
                channels,
                image.get_width() as i32 * channels,
            )?
            .set_source_resolution(96);
        let result = tesseract.get_text()?;
        let _tesseract = mutex_guard.insert(tesseract);
        Ok(result)
    }
}

impl Default for TesseractTrigger {
    fn default() -> Self {
        Self::new()
    }
}

/// Attempts to download the latest traineddata file from Github
pub fn download_tesseract_traineddata(download_path: &Path) -> Result<()> {
    // Download latest from Github
    let filename = download_path
        .file_name()
        .ok_or(NoneError)?
        .to_str()
        .ok_or(NoneError)?;
    let url = format!(
        "https://github.com/tesseract-ocr/tessdata/raw/4.00/{}",
        filename
    );
    let body = reqwest::blocking::get(url)?.bytes()?;

    // Automatically create needed directories
    fs::create_dir_all(download_path.parent().unwrap())?;

    // Write to file
    let mut file = File::create(download_path)?;
    Ok(file.write_all(body.as_ref())?)
}

pub type TesseractRef = Arc<Mutex<Option<Tesseract>>>;

/// Initialize a Tesseract instance, automatically downloading traineddata if needed
pub fn init_tesseract<'a, X, Y>(datapath: X, language: Y) -> Result<TesseractRef>
where
    X: Into<Option<&'a str>>,
    Y: Into<Option<&'a str>>,
{
    let current_exe = std::env::current_exe()?;
    let default_datapath_pathbuf = current_exe.parent().ok_or(NoneError)?.join(""); // this fixed something???
    let default_datapath = default_datapath_pathbuf
        .as_os_str()
        .to_str()
        .ok_or(NoneError)?;
    let datapath = Into::<Option<&str>>::into(datapath).unwrap_or(default_datapath);
    let language = Into::<Option<&str>>::into(language).unwrap_or("eng");
    println!("[tesseract] using datapath {}", datapath);
    println!("[tesseract] using language {}", language);

    // Check for trainedata and try downloading if needed
    let traineddata_pathbuf = Path::new(datapath).join(format!("{}.traineddata", language));
    let traineddata_path = traineddata_pathbuf.as_path();
    if !traineddata_path.exists() {
        println!(
            "[tesseract] could not find traineddata at {}",
            traineddata_path.display()
        );
        println!("[tesseract] downloading traineddata...");
        download_tesseract_traineddata(traineddata_path)?;
        println!("[tesseract] traineddata downloaded!");
    } else {
        println!("[tesseract] found traineddata")
    }

    let tesseract = Tesseract::new(Some(datapath), Some(language))?;
    Ok(Arc::new(Mutex::new(Some(tesseract))))
}

#[cfg(test)]
mod tests {
    use super::{init_tesseract, TesseractTrigger};
    use crate::async_trigger::{AsyncTrigger, TriggerThread};
    use crate::error::{Error, Result};
    use crate::photon::Crop;
    use crate::pipeline::Hypetrigger;

    #[test]
    fn tesseract() -> Result<()> {
        let tesseract = init_tesseract(None, None)?;
        let trigger = TesseractTrigger {
            tesseract,
            crop: Some(Crop {
                left_percent: 25.0,
                top_percent: 25.0,
                width_percent: 10.0,
                height_percent: 10.0,
            }),
            threshold_filter: None,
            callback: None,
            enable_debug_breakpoints: false,
        };

        Hypetrigger::new()
            .test_input()
            .add_trigger(trigger)
            .run()
            .map_err(Error::from_display)
    }

    #[test]
    fn async_trigger() -> Result<()> {
        let runner_thread = TriggerThread::spawn();
        let tesseract = init_tesseract(None, None)?;
        let base_trigger = TesseractTrigger {
            tesseract,
            crop: Some(Crop {
                left_percent: 25.0,
                top_percent: 25.0,
                width_percent: 10.0,
                height_percent: 10.0,
            }),
            threshold_filter: None,
            callback: None,
            enable_debug_breakpoints: false,
        };
        let trigger = AsyncTrigger::from_trigger(base_trigger, runner_thread);

        Hypetrigger::new()
            .test_input()
            .add_trigger(trigger)
            .run()
            .map_err(Error::from_display)
    }
}
