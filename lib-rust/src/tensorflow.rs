use crate::{
    config::HypetriggerConfig,
    emit::OnEmit,
    photon::{ensure_size, ensure_square, rgb24_to_rgba32},
    runner::{RunnerCommand, RunnerFn, RunnerResult},
    trigger::{self, Crop, Trigger, TriggerParams, Triggers},
};
use photon_rs::PhotonImage;
use std::{
    collections::HashMap,
    env::current_exe,
    path::PathBuf,
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

pub struct TensorflowParams {
    pub model_dir: String,
}

impl TriggerParams for TensorflowParams {
    fn get_runner_type(&self) -> String {
        TENSORFLOW_RUNNER.into()
    }
}

/// - Receives: either an image to process, or an exit command
/// - Sends: the image classification label from Tensorflow
pub fn tensorflow_runner(
    rx: Receiver<RunnerCommand>,
    on_result: OnEmit,
    config: Arc<HypetriggerConfig>,
) {
    let saved_models: ModelMap = init_tensorflow(&config.triggers);
    // let mut consecutive_matches = init_consecutive_matches(&context.config.triggers);

    while let Ok(command) = rx.recv() {
        match command {
            RunnerCommand::ProcessImage(payload) => {
                let trigger = payload.trigger;

                // 0. Get corresponding model
                let (bundle, graph) = saved_models.get(&trigger.id).expect("get model");

                // 1. convert raw image to photon
                let vector = Arc::try_unwrap(payload.image).expect("unwrap buffer");
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
                let prediction = predict(bundle, graph, &tensor);
                // todo!("retrieve label name");
                // todo!("retrieve confidence values");
                let text: String = prediction.to_string();

                // 4. forward results to tx
                let result = RunnerResult {
                    text,
                    trigger_id: trigger.id.clone(),
                    input_id: payload.input_id.clone(),
                    frame_num: 0,
                    timestamp: 0,
                };
                on_result(result);
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
        if trigger.params.get_runner_type() != TENSORFLOW_RUNNER {
            eprintln!("trigger {} is not a tensorflow trigger", trigger.id);
            continue;
        }

        let tensorflow_params = trigger
            .params
            .as_any()
            .downcast_ref::<Arc<TensorflowParams>>()
            .unwrap();

        let saved_model_path: PathBuf = current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(tensorflow_params.model_dir.clone());
        let save_model_path_str: &str = saved_model_path.as_os_str().to_str().to_owned().unwrap();

        // todo!("logging config");
        println!("[tensorflow] saved model path = {}", save_model_path_str);

        saved_models.insert(
            trigger.id.clone(),
            load_tensorflow_model(save_model_path_str),
        );
    }

    saved_models
}

pub fn load_tensorflow_model(save_dir: &str) -> (SavedModelBundle, Graph) {
    println!("[tensorflow] Loading saved model");
    let now = Instant::now();

    let mut graph = Graph::new();
    let bundle = SavedModelBundle::load(&SessionOptions::new(), ["serve"], &mut graph, save_dir)
        .expect("load model bundle");

    println!(
        "[tensorflow] load_tensorflow_model {}ms.",
        now.elapsed().as_millis()
    );

    // Initialize the session by running a dummy input through the graph.
    let dummy = dummy_tensor();
    predict(&bundle, &graph, &dummy);
    println!("[tensorflow] finished test run");

    (bundle, graph)
}

/// Creates a tensor of zeros (all black image) used to initialize a session for fast prediction.
pub fn dummy_tensor() -> Tensor<f32> {
    let bytes = TENSOR_SIZE * TENSOR_SIZE * TENSOR_CHANNELS;
    let zero_vec: Vec<f32> = vec![0 as f32; bytes as usize];

    Tensor::new(&[1, TENSOR_SIZE, TENSOR_SIZE, TENSOR_CHANNELS])
        .with_values(&zero_vec)
        .expect("creating dummy tensor")
}

pub fn predict(bundle: &SavedModelBundle, graph: &Graph, tensor: &Tensor<f32>) -> usize {
    let mut args = SessionRunArgs::new();

    // get in/out operations
    let signature = bundle
        .meta_graph_def()
        .get_signature(tensorflow::DEFAULT_SERVING_SIGNATURE_DEF_KEY)
        .expect("Get signature");
    let x_info = signature.get_input("Image").expect("Get image input");
    let op_x = &graph
        .operation_by_name_required(&x_info.name().name)
        .expect("Get input name");
    let output_info = signature.get_output("Confidences").expect("Get output");
    let op_output = &graph
        .operation_by_name_required(&output_info.name().name)
        .expect("Get output name");

    // Load our input image
    args.add_feed(op_x, 0, tensor);
    let token_output = args.request_fetch(op_output, 0);

    // Run prediction
    let session = &bundle.session;
    session.run(&mut args).expect("run prediction");

    // Check the output.
    let output: Tensor<f32> = args.fetch(token_output).expect("fetch output");

    // Calculate argmax of the output)
    let (max_idx, _max_val) =
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

    max_idx
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
