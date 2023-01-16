use crate::{
    debug::debug_photon_image,
    error::Result,
    photon::{ensure_size, ensure_square, rgb_to_photon, rgba32_to_rgb24, Crop},
    trigger::{Frame, Trigger},
};
use photon_rs::PhotonImage;
use std::{path::Path, time::Instant};
use tensorflow::{Graph, SavedModelBundle, SessionOptions, SessionRunArgs, Tensor};

/// Side length of the square image that the model expects
pub const TENSOR_SIZE: u64 = 224;

/// Color channels expected (RGB)
pub const TENSOR_CHANNELS: u64 = 3;

/// The key in the hashmap of Runners, used to map Triggers to their Runners
pub const TENSORFLOW_RUNNER: &str = "tensorflow";

pub type TensorflowTriggerCallback = Box<dyn Fn(&Prediction) + Send + Sync>;

pub struct TensorflowTrigger {
    pub crop: Option<Crop>,
    pub bundle: SavedModelBundle,
    pub graph: Graph,
    pub callback: Option<TensorflowTriggerCallback>,
}

impl Trigger for TensorflowTrigger {
    fn on_frame(&self, frame: &Frame) -> Result<()> {
        // 1. convert raw image to photon
        let image = rgb_to_photon(&frame.image);

        // 2. preprocess
        let filtered = self.preprocess_image(image)?;

        // 3. image classification
        let rgba32 = filtered.get_raw_pixels();
        let rgb24 = rgba32_to_rgb24(rgba32);
        let buf = rgb24.as_slice();
        let tensor = buffer_to_tensor(buf);
        let prediction = predict(&self.bundle, &self.graph, &tensor)?;

        // 4. callback
        if let Some(callback) = &self.callback {
            callback(&prediction);
        }

        Ok(())
    }
}

impl TensorflowTrigger {
    pub fn new<P>(
        model_dir: P,
        crop: Option<Crop>,
        callback: Option<TensorflowTriggerCallback>,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let (bundle, graph) = load_tensorflow_model(model_dir)?;
        Ok(Self {
            bundle,
            graph,
            crop,
            callback,
        })
    }

    pub fn preprocess_image(&self, mut image: PhotonImage) -> Result<PhotonImage> {
        /// If `true`, pauses execution after each step of image pre-processing.
        const DEBUG: bool = false;
        if DEBUG {
            println!("[tensorflow] received frame");
            debug_photon_image(&image)?;
        }

        if let Some(crop) = &self.crop {
            image = crop.apply(image);
        }

        let size = TENSOR_SIZE as u32;
        image = ensure_square(image);
        image = ensure_size(image, size, size);

        if DEBUG {
            println!("[tensorflow] center square crop and resize to 224x224 px");
            debug_photon_image(&image)?;
        }

        debug_assert!(image.get_width() == size);
        debug_assert!(image.get_height() == size);

        Ok(image)
    }
}

pub fn load_tensorflow_model<P>(model_dir: P) -> Result<(SavedModelBundle, Graph)>
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
    predict(&bundle, &graph, &dummy)?;
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
) -> Result<Prediction> {
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

    for pixel in buf.iter().take(bytes as usize) {
        flattened.push(*pixel as f32 / 255.0);
    }

    Tensor::new(&[1, TENSOR_SIZE, TENSOR_SIZE, TENSOR_CHANNELS])
        .with_values(&flattened)
        .expect("creating tensor from buffer")
}

// /// 1. (if needed) Center crop if not square
// /// 2. (if needed) Resize to 224x224
// #[wasm_bindgen]
// pub fn preprocess_image_for_tensorflow(image: PhotonImage) -> PhotonImage {
//     let size = TENSOR_SIZE as u32;
//     let mut new_image: PhotonImage = image;
//     new_image = ensure_square(new_image);
//     new_image = ensure_size(new_image, size, size);

//     debug_assert!(new_image.get_width() == size);
//     debug_assert!(new_image.get_height() == size);

//     new_image
// }
