use crate::debug::debug_photon_image;
use crate::error::{NoneError, Result};
use crate::photon::ImageTransform;
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

#[derive(Clone)]
pub struct TesseractTrigger {
    pub tesseract: Arc<Mutex<Option<Tesseract>>>,
    pub crop: Option<Crop>,
    pub threshold_filter: Option<ThresholdFilter>,
    pub callback: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

impl Trigger for TesseractTrigger {
    fn on_frame(&self, frame: &Frame) -> Result<()> {
        // 1. convert raw image to photon
        let image = rgb_to_photon(&frame.image);

        // 2. preprocess
        let filtered = self.preprocess_image(image);

        // 3. run ocr
        let text = self.ocr(filtered)?;

        // 4. callback
        if let Some(callback) = &self.callback {
            callback(text.as_str());
        }

        Ok(())
    }
}

impl TesseractTrigger {
    pub fn preprocess_image(&self, mut image: PhotonImage) -> PhotonImage {
        /// If `true`, pauses execution after each step of image pre-processing.
        const DEBUG: bool = false;
        if DEBUG {
            println!("[tesseract] received frame");
            debug_photon_image(&image);
        }

        // Crop
        if let Some(crop) = &self.crop {
            image = crop.apply(image);
        }
        if DEBUG {
            println!("[tesseract] cropped");
            debug_photon_image(&image);
        }

        // Minimum size
        const MIN_TESSERACT_IMAGE_SIZE: u32 = 32;
        image = ensure_minimum_size(&image, MIN_TESSERACT_IMAGE_SIZE);
        if DEBUG {
            println!("[tesseract] resized");
            debug_photon_image(&image);
        }

        // Threshold filter
        if let Some(filter) = &self.threshold_filter {
            image = filter.apply(image);
        }
        if DEBUG {
            println!("[tesseract] filtered");
            debug_photon_image(&image);
        }

        // Padding
        let padding_bg: Rgba = Rgba::new(255, 255, 255, 255);
        image = padding_uniform(&image, MIN_TESSERACT_IMAGE_SIZE, padding_bg);
        if DEBUG {
            println!("[tesseract] padded (done)");
            debug_photon_image(&image);
        }

        image
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
        mutex_guard.insert(tesseract);
        Ok(result)
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

/// Initialize a Tesseract instance, automatically downloading traineddata if needed
pub fn init_tesseract<'a, X, Y>(datapath: X, language: Y) -> Result<Arc<Mutex<Option<Tesseract>>>>
where
    X: Into<Option<&'a str>>,
    Y: Into<Option<&'a str>>,
{
    let current_exe = std::env::current_exe()?;
    let default_datapath_pathbuf = current_exe.parent().unwrap().join(""); // this fixed something???
    let default_datapath = default_datapath_pathbuf.as_os_str().to_str().unwrap();
    let datapath = Into::<Option<&str>>::into(datapath).unwrap_or(default_datapath.as_ref());
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

// /// Either apply a filter or pass through the original image
// pub fn try_filter(image: PhotonImage, filter: Option<ThresholdFilter>) -> PhotonImage {
//     match filter {
//         Some(filter) => {
//             let color = Rgb::new(filter.r, filter.g, filter.b);
//             threshold_color_distance(image, &color, filter.threshold)
//         }
//         _ => image,
//     }
// }

// /// 1. (optional) Resize if smaller than 32px in either dimension
// /// 2. (optional) Apply threshold filter if present
// /// 3. Add 32px padding to all sides for better OCR results
// #[wasm_bindgen]
// pub fn preprocess_image_for_tesseract(
//     image: &PhotonImage,
//     filter: Option<ThresholdFilter>,
// ) -> PhotonImage {
//     // Tesseract struggles to recognize text below this size.
//     const MIN_TESSERACT_IMAGE_SIZE: u32 = 32;

//     // Color for background padding
//     let background: Rgba = Rgba::new(255, 255, 255, 255);

//     // Chain image transformations
//     // NB: Is there a fancy pipe() + partial application operator for this? It would
//     // need to bind the output of one function to the input of the next, while
//     // allowing additional subsequent arguments.
//     let mut new_image: PhotonImage;
//     new_image = ensure_minimum_size(image, MIN_TESSERACT_IMAGE_SIZE);
//     new_image = try_filter(new_image, filter);
//     new_image = padding_uniform(&new_image, MIN_TESSERACT_IMAGE_SIZE, background);
//     new_image
// }

// /// Recognize text from an image
// pub fn ocr(image: PhotonImage, tesseract: &RefCell<Option<Tesseract>>) -> String {
//     let _now = Instant::now();
//     let rgba32 = image.get_raw_pixels();
//     let buf = rgba32.as_slice();
//     let channels = 4;

//     let mut model = tesseract.replace(None).unwrap();
//     model = model
//         .set_frame(
//             buf,
//             image.get_width() as i32,
//             image.get_height() as i32,
//             channels,
//             image.get_width() as i32 * channels,
//         )
//         .expect("load image from memory")
//         .set_source_resolution(96);
//     let result = model.get_text().expect("get text");
//     tesseract.replace(Some(model));

//     result
// }
