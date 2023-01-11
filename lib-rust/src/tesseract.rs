use crate::{
    config::HypetriggerConfig,
    debugger::{Debugger, DebuggerStep},
    emit::OnEmitV2,
    photon::{ensure_minimum_size, rgb24_to_rgba32},
    pipeline::OnPanic,
    pipeline_simple::{Error, NoneError},
    runner::{RunnerCommand, RunnerResultV2},
    threshold::threshold_color_distance,
    trigger::{Crop, Trigger},
};
use photon_rs::{
    helpers::{self, dyn_image_from_raw},
    transform::padding_uniform,
    PhotonImage, Rgb, Rgba,
};
use std::{
    cell::RefCell,
    fs::{self, File},
    io::{self, stdin, Write},
    path::{Path, PathBuf},
    sync::{mpsc::Receiver, Arc, Mutex},
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
pub fn tesseract_runner(
    rx: Receiver<RunnerCommand>,
    _config: Arc<HypetriggerConfig>,
    on_panic: OnPanic,
) {
    let tesseract = init_tesseract(None, None).unwrap();
    println!("[tesseract] thread initialized");

    let mut i = 0;
    while let Ok(command) = rx.recv() {
        i += 1;
        match command {
            RunnerCommand::ProcessImage(context, debugger_ref) => {
                // -1. unwrap debugger ref
                let debugger = match debugger_ref.read() {
                    Ok(debugger) => debugger,
                    Err(e) => {
                        return on_panic(Box::new(io::Error::new(
                            io::ErrorKind::Other,
                            e.to_string(),
                        )))
                    }
                };

                // 0. downcast to concrete Trigger type
                let trigger = match context.trigger.as_any().downcast_ref::<TesseractTrigger>() {
                    Some(trigger) => trigger,
                    None => {
                        return on_panic(Box::new(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Tesseract runner received a non-Tesseract trigger!",
                        )))
                    }
                };
                let input_id = context.config.inputPath.clone();
                let frame_num = context.frame_num;
                let timestamp = context.get_timestamp();

                // 0.5. unwrap buffer from ffmpeg
                let vector = match Arc::try_unwrap(context.image) {
                    Ok(vector) => vector,
                    Err(arc) => {
                        println!("[err] try_unwrap failed on RawImageData Arc");
                        eprintln!(
                          "[err] This indicates that there's more than one strong reference to the Arc"
                      );
                        eprintln!("[err] the Arc's internal buffer has length {}", arc.len());
                        return on_panic(Box::new(io::Error::new(
                            io::ErrorKind::Other,
                            "could not unwrap RawImageData Arc (referenced elsewhere)",
                        )));
                    }
                };
                debugger.log(&format!("[tesseract] Received {} bytes", vector.len()));

                // 1. convert raw image to photon
                let rgba32 = rgb24_to_rgba32(vector);
                let image = PhotonImage::new(rgba32, trigger.crop.width, trigger.crop.height);

                // Register a potential breakpoint
                let dyn_image = helpers::dyn_image_from_raw(&image).to_rgb8();
                Debugger::register_step(
                    debugger_ref.clone(),
                    DebuggerStep {
                        config: context.config.clone(),
                        trigger: context.trigger.clone(),
                        frame_num: context.frame_num,
                        description: "Tesseract received image".into(),
                        image: Some(dyn_image),
                    },
                );

                // 2. preprocess
                let filtered = preprocess_image_for_tesseract(&image, trigger.filter.clone());

                // 3. run ocr
                // let text = ocr(filtered, &tesseract);
                let text = "deprecated".to_string();

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

/// Attempts to download the latest traineddata file from Github
pub fn download_tesseract_traineddata(download_path: &Path) -> Result<(), Error> {
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
pub fn init_tesseract(
    datapath: Option<&str>,
    language: Option<&str>,
) -> Result<Arc<Mutex<Option<Tesseract>>>, Error> {
    let current_exe = std::env::current_exe()?;
    let default_datapath_pathbuf = current_exe.parent().unwrap().join(""); // this fixed something???
    let default_datapath = default_datapath_pathbuf.as_os_str().to_str().unwrap();
    let datapath = datapath.unwrap_or(default_datapath.as_ref());
    let language = language.unwrap_or("eng");
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
        download_tesseract_traineddata(traineddata_path)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;
        println!("[tesseract] traineddata downloaded!");
    } else {
        println!("[tesseract] found traineddata")
    }

    let tesseract = Tesseract::new(Some(datapath), Some(language))?;
    Ok(Arc::new(Mutex::new(Some(tesseract))))
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
