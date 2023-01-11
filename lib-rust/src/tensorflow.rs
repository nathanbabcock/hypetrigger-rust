use crate::{
    config::HypetriggerConfig,
    emit::OnEmitV2,
    photon::{ensure_size, ensure_square, rgb24_to_rgba32},
    pipeline::OnPanic,
    runner::{RunnerCommand, RunnerResultV2},
    trigger::{Crop, Trigger, Triggers},
};
use photon_rs::PhotonImage;
use std::{
    collections::HashMap,
    env::current_exe,
    path::{Path, PathBuf},
    sync::{mpsc::Receiver, Arc},
    time::Instant,
};
use tensorflow::{Graph, SavedModelBundle, SessionOptions, SessionRunArgs, Tensor};
use wasm_bindgen::prelude::wasm_bindgen;

/// Side length of the square image that the model expects
pub const TENSOR_SIZE: u64 = 224;

/// Color channels expected (RGB)
pub const TENSOR_CHANNELS: u64 = 3;

/// The key in the hashmap of Runners, used to map Triggers to their Runners
pub const TENSORFLOW_RUNNER: &str = "tensorflow";

pub type ModelMap = HashMap<String, (SavedModelBundle, Graph)>;

pub struct TensorflowTrigger {
    // pub id: String,
    pub crop: Crop,
    pub model_dir: String,
    pub on_emit: OnEmitV2<String>,
}

impl Trigger for TensorflowTrigger {
    fn get_runner_type(&self) -> String {
        TENSORFLOW_RUNNER.into()
    }

    fn get_crop(&self) -> Crop {
        self.crop.clone()
    }
}

/// - Receives: either an image to process, or an exit command
/// - Sends: the image classification label from Tensorflow
pub fn tensorflow_runner(
    rx: Receiver<RunnerCommand>,
    config: Arc<HypetriggerConfig>,
    _on_panic: OnPanic,
) {
    let saved_models: ModelMap = init_tensorflow(&config.triggers);
    // let mut consecutive_matches = init_consecutive_matches(&context.config.triggers);

    // TODO use `on_panic` to gracefully shut down on fatal errors

    while let Ok(command) = rx.recv() {
        match command {
            RunnerCommand::ProcessImage(context, _debugger) => {
                // -1. Downcast to concrete Trigger type
                let trigger = context
                    .trigger
                    .as_any()
                    .downcast_ref::<TensorflowTrigger>()
                    .expect("Tensorflow runner received a non-Tensorflow trigger!");

                // 0. Get corresponding model
                let (bundle, graph) = saved_models.get(&trigger.model_dir).expect("get model");

                // 1. convert raw image to photon
                let input_id = context.config.inputPath.clone();
                let frame_num = context.frame_num;
                let timestamp = context.get_timestamp();
                let vector = Arc::try_unwrap(context.image).expect("unwrap buffer");
                let rgba32 = rgb24_to_rgba32(vector);
                let image = PhotonImage::new(rgba32, trigger.crop.width, trigger.crop.height);

                // 2. preprocess
                let filtered = preprocess_image_for_tensorflow(image);

                // 3. run inference
                let rgb32 = filtered.get_raw_pixels();
                let rgb24 = {
                    let vec = rgb32;
                    let mut new_vec = Vec::with_capacity(vec.len() * 3 / 4);
                    for i in (0..vec.len()).step_by(4) {
                        new_vec.push(vec[i]);
                        new_vec.push(vec[i + 1]);
                        new_vec.push(vec[i + 2]);
                    }
                    new_vec
                };
                let buf = rgb24.as_slice();
                let tensor = buffer_to_tensor(buf);
                let prediction = predict(bundle, graph, &tensor).unwrap().class_index;
                // todo!("retrieve label name");
                // todo!("retrieve confidence values");
                let text: String = prediction.to_string();

                // 4. forward results to tx
                let result = RunnerResultV2 {
                    result: text,
                    trigger_id: "".into(), // no longer exists // trigger.id.clone(),
                    input_id,
                    frame_num,
                    timestamp,
                };

                // 5. emit
                (trigger.on_emit)(result);
            }
            RunnerCommand::Exit => {
                println!("[tensorflow] received exit command");
                break;
            }
        }
    }
}

pub fn init_tensorflow(triggers: &Triggers) -> ModelMap {
    let mut saved_models: ModelMap = HashMap::new();
    for trigger in triggers {
        if trigger.get_runner_type() != TENSORFLOW_RUNNER {
            // eprintln!("trigger {} is not a tensorflow trigger", trigger.id);
            continue;
        }

        let trigger = trigger
            .as_any()
            .downcast_ref::<TensorflowTrigger>()
            .unwrap();

        let saved_model_path: PathBuf = current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(trigger.model_dir.clone());
        let save_model_path_str: &str = saved_model_path.as_os_str().to_str().to_owned().unwrap();

        // todo!("logging config");
        println!("[tensorflow] saved model path = {}", save_model_path_str);

        saved_models.insert(
            trigger.model_dir.clone(),
            load_tensorflow_model(save_model_path_str).unwrap(),
        );
    }

    saved_models
}

pub fn load_tensorflow_model<P>(
    model_dir: P,
) -> Result<(SavedModelBundle, Graph), crate::pipeline_simple::Error>
where
    P: AsRef<Path>,
{
    println!("[tensorflow] Loading saved model");
    let now = Instant::now();

    let mut graph = Graph::new();
    let bundle = SavedModelBundle::load(&SessionOptions::new(), ["serve"], &mut graph, model_dir)?;

    println!(
        "[tensorflow] load_tensorflow_model {}ms.",
        now.elapsed().as_millis()
    );

    // Initialize the session by running a dummy input through the graph.
    let dummy = dummy_tensor();
    predict(&bundle, &graph, &dummy);
    println!("[tensorflow] finished test run");

    Ok((bundle, graph))
}

/// Creates a tensor of zeros (all black image) used to initialize a session for fast prediction.
pub fn dummy_tensor() -> Tensor<f32> {
    let bytes = TENSOR_SIZE * TENSOR_SIZE * TENSOR_CHANNELS;
    let zero_vec: Vec<f32> = vec![0 as f32; bytes as usize];

    Tensor::new(&[1, TENSOR_SIZE, TENSOR_SIZE, TENSOR_CHANNELS])
        .with_values(&zero_vec)
        .expect("creating dummy tensor")
}

pub struct Prediction {
    /// The index of the class with the highest confidence.
    pub class_index: usize,

    // pub label: String, // TODO
    /// Confidence interval in the prediction, in the range [0, 1].
    pub confidence: f32,
}

pub fn predict(
    bundle: &SavedModelBundle,
    graph: &Graph,
    tensor: &Tensor<f32>,
) -> Result<Prediction, crate::pipeline_simple::Error> {
    let mut args = SessionRunArgs::new();

    // get in/out operations
    let meta_graph_def = bundle.meta_graph_def();
    let signature = meta_graph_def.get_signature(tensorflow::DEFAULT_SERVING_SIGNATURE_DEF_KEY)?;
    let x_info = signature.get_input("Image")?;
    let op_x = &graph.operation_by_name_required(&x_info.name().name)?;
    let output_info = signature.get_output("Confidences")?;
    let op_output = &graph.operation_by_name_required(&output_info.name().name)?;

    // Load our input image
    args.add_feed(op_x, 0, tensor);
    let token_output = args.request_fetch(op_output, 0);

    // Run prediction
    let session = &bundle.session;
    session.run(&mut args)?;

    // Check the output.
    let output: Tensor<f32> = args.fetch(token_output)?;

    // Calculate argmax of the output)
    let (max_idx, max_val) =
        output
            .iter()
            .enumerate()
            .fold((0, output[0]), |(idx_max, val_max), (idx, val)| {
                if &val_max > val {
                    (idx_max, val_max)
                } else {
                    (idx, *val)
                }
            });

    Ok(Prediction {
        class_index: max_idx,
        confidence: max_val,
    })
}

pub fn buffer_to_tensor(buf: &[u8]) -> Tensor<f32> {
    let mut flattened: Vec<f32> = Vec::new();
    let bytes = TENSOR_SIZE * TENSOR_SIZE * TENSOR_CHANNELS;
    for i in 0..bytes as usize {
        flattened.push(buf[i] as f32 / 255.0);
    }

    Tensor::new(&[1, TENSOR_SIZE, TENSOR_SIZE, TENSOR_CHANNELS])
        .with_values(&flattened)
        .expect("creating tensor from buffer")
}

/// 1. (if needed) Center crop if not square
/// 2. (if needed) Resize to 224x224
#[wasm_bindgen]
pub fn preprocess_image_for_tensorflow(image: PhotonImage) -> PhotonImage {
    let size = TENSOR_SIZE as u32;
    let mut new_image: PhotonImage = image;
    new_image = ensure_square(new_image);
    new_image = ensure_size(new_image, size, size);

    debug_assert!(new_image.get_width() == size);
    debug_assert!(new_image.get_height() == size);

    new_image
}
