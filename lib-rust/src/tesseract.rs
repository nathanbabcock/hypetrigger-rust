use crate::{
    config::HypetriggerConfig,
    emit::{OnEmitV2},
    photon::{ensure_minimum_size, rgb24_to_rgba32},
    runner::{RunnerCommand, RunnerResultV2},
    threshold::threshold_color_distance,
    trigger::{Crop, Trigger},
};
use photon_rs::{transform::padding_uniform, PhotonImage, Rgb, Rgba};
use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{mpsc::Receiver, Arc},
    time::Instant,
};
use tesseract::{InitializeError, Tesseract};
use wasm_bindgen::prelude::wasm_bindgen;

pub struct TesseractTrigger {
    // pub id: String, // here, or in trait?
    pub crop: Crop, // we may choose to crop in the runner, instead of in ffmpeg filter
    pub filter: Option<ThresholdFilter>,
    pub on_emit: OnEmitV2<String>,
}

/// The key in the hashmap of Runners, used to map Triggers to their Runners
pub const TESSERACT_RUNNER: &str = "tesseract";
impl Trigger for TesseractTrigger {
    fn get_runner_type(&self) -> String {
        TESSERACT_RUNNER.into()
    }

    fn get_crop(&self) -> Crop {
        self.crop.clone()
    }
}

#[derive(Clone, Debug)]
#[wasm_bindgen]
pub struct ThresholdFilter {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub threshold: f64,
}

/// - Receives: either an image to process, or an exit command
/// - Sends: the recognized text
pub fn tesseract_runner(rx: Receiver<RunnerCommand>, _config: Arc<HypetriggerConfig>) {
    let tesseract = RefCell::new(Some(init_tesseract().unwrap()));
    println!("[tesseract] thread initialized");

    while let Ok(command) = rx.recv() {
        match command {
            RunnerCommand::ProcessImage(context) => {
                // 0. downcast to concrete Trigger type
                let trigger = context
                    .trigger
                    .as_any()
                    .downcast_ref::<TesseractTrigger>()
                    .expect("Tesseract runner received a non-Tesseract trigger!");

                // 1. convert raw image to photon
                let input_id = context.config.inputPath.clone();
                let frame_num = context.frame_num;
                let timestamp = context.get_timestamp();
                let vector = Arc::try_unwrap(context.image).expect("unwrap buffer");
                let rgba32 = rgb24_to_rgba32(vector);
                let image = PhotonImage::new(rgba32, trigger.crop.width, trigger.crop.height);

                // 2. preprocess
                let filtered = preprocess_image_for_tesseract(&image, trigger.filter.clone());

                // 3. run ocr
                let text = ocr(filtered, &tesseract);

                // 4. forward results to tx
                let result = RunnerResultV2 {
                    result: text,
                    trigger_id: "".into(), //trigger.id.clone(), // TODO we removed this from Trigger Trait -- restore?
                    input_id,
                    frame_num,
                    timestamp,
                };

                // 5. emit/callback
                (trigger.on_emit)(result);

                // // todo!("optional logging");
                // println!(
                //     "[tesseract] {} ({}ms) => {}",
                //     trigger_id.unwrap_or("unknown trigger".into()),
                //     now.elapsed().as_millis(),
                //     result.trim(),
                // );
            }
            RunnerCommand::Exit => {
                println!("[tesseract] received exit command");
                break;
            }
        }
    }

    println!("[tesseract] thread exiting");
}

pub fn init_tesseract() -> Result<Tesseract, InitializeError> {
    let tessdata_path: PathBuf = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("data\\tessdata");
    let datapath: Option<&str> = tessdata_path.as_os_str().to_str();
    println!("[tesseract] tessdata = {}", datapath.unwrap());

    const LANGUAGE: Option<&str> = Some("eng");

    Tesseract::new(datapath, LANGUAGE)
}

/// Either apply a filter or pass through the original image
pub fn try_filter(image: PhotonImage, filter: Option<ThresholdFilter>) -> PhotonImage {
    match filter {
        Some(filter) => {
            let color = Rgb::new(filter.r, filter.g, filter.b);
            threshold_color_distance(image, &color, filter.threshold)
        }
        _ => image,
    }
}

/// 1. (optional) Resize if smaller than 32px in either dimension
/// 2. (optional) Apply threshold filter if present
/// 3. Add 32px padding to all sides for better OCR results
#[wasm_bindgen]
pub fn preprocess_image_for_tesseract(
    image: &PhotonImage,
    filter: Option<ThresholdFilter>,
) -> PhotonImage {
    // Tesseract struggles to recognize text below this size.
    const MIN_TESSERACT_IMAGE_SIZE: u32 = 32;

    // Color for background padding
    let background: Rgba = Rgba::new(255, 255, 255, 255);

    // Chain image transformations
    // NB: Is there a fancy pipe() + partial application operator for this? It would
    // need to bind the output of one function to the input of the next, while
    // allowing additional subsequent arguments.
    let mut new_image: PhotonImage;
    new_image = ensure_minimum_size(image, MIN_TESSERACT_IMAGE_SIZE);
    new_image = try_filter(new_image, filter);
    new_image = padding_uniform(&new_image, MIN_TESSERACT_IMAGE_SIZE, background);
    new_image
}

/// Recognize text from an image
pub fn ocr(image: PhotonImage, tesseract: &RefCell<Option<Tesseract>>) -> String {
    let _now = Instant::now();
    let rgba32 = image.get_raw_pixels();
    let buf = rgba32.as_slice();
    let channels = 4;

    let mut model = tesseract.replace(None).unwrap();
    model = model
        .set_frame(
            buf,
            image.get_width() as i32,
            image.get_height() as i32,
            channels,
            image.get_width() as i32 * channels,
        )
        .expect("load image from memory")
        .set_source_resolution(96);
    let result = model.get_text().expect("get text");
    tesseract.replace(Some(model));

    result
}
