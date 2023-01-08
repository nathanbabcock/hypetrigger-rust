use photon_rs::PhotonImage;
use tesseract::Tesseract;

use crate::config::HypetriggerConfig;
use crate::ffmpeg::RawImageData;
use crate::tesseract::init_tesseract;
use std::os::windows::process::CommandExt;
use std::process::Stdio;
use std::sync::Mutex;
use std::{
    io,
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
};

//// Image processing

pub struct ThresholdFilter {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub threshold: u8,
}

impl ThresholdFilter {
    pub fn filter_image(&self, image: PhotonImage) {
        todo!();
    }
}

pub struct Crop {
    pub left_percent: f64,
    pub top_percent: f64,
    pub width_percent: f64,
    pub height_percent: f64,
}

impl Crop {
    pub fn crop_image(&self, image: PhotonImage) {
        todo!();
    }
}

//// Triggers
pub trait Trigger {
    fn run(&self, frame: RawImageData) -> Result<(), String>;
}

pub struct TesseractTrigger {
    tesseract: Mutex<Tesseract>,
    crop: Crop,
    threshold_filter: ThresholdFilter,
}

impl Trigger for TesseractTrigger {
    fn run(&self, frame: RawImageData) -> Result<(), String> {
        Err("not implemented".to_string())
    }
}

// //// Job (remove this?)
// pub struct HypetriggerJob {
//     pub config: HypetriggerConfig,
//     pub ffmpeg_child: Child,
//     pub ffmpeg_stdin: Option<ChildStdin>,
//     pub ffmpeg_stdout: Option<ChildStdout>,
//     pub ffmpeg_stderr: Option<ChildStderr>,
// }

//// Pipeline
#[derive(Default)]
pub struct Hypetrigger {
    // Path the the ffmpeg binary or command to use
    pub ffmpeg_exe: String,

    /// Path to input video (or image) for ffmpeg
    pub input: String,

    /// Framerate to sample the input video at.
    /// This can (an should) by much lower than the input video's native framerate.
    /// 2-4 frames per second is more than sufficient to capture most events.
    pub fps: u64,

    /// List of all callback functions to run on each frame of the video
    pub triggers: Vec<Box<dyn Trigger>>,
}

impl Hypetrigger {
    // --- Getters and setters ---
    /// Setter for the ffmpeg binary or command to use
    pub fn set_ffmpeg_exe(&mut self, ffmpeg_exe: String) -> &mut Self {
        self.ffmpeg_exe = ffmpeg_exe;
        self
    }

    /// Setter for the input video (or image) for ffmpeg
    pub fn set_input(&mut self, input: String) -> &mut Self {
        self.input = input;
        self
    }

    /// Setter for the framerate to sample the input video at.
    pub fn set_fps(&mut self, fps: u64) -> &mut Self {
        self.fps = fps;
        self
    }

    /// Add a Trigger to be run on every frame of the input
    pub fn add_trigger(&mut self, trigger: Box<dyn Trigger>) -> &mut Self {
        self.triggers.push(trigger);
        self
    }

    // --- Constructor ---
    pub fn new() -> Self {
        Self::default()
    }

    // --- Behavior ---
    /// Spawn ffmpeg, call callbacks on each frame, and block until completion.
    pub fn run(&self) -> Result<(), String> {
        Err("Not implemented".to_string())
    }
}

pub fn _main() -> Result<(), String> {
    let tesseract = Mutex::new(match init_tesseract() {
        Ok(tesseract) => tesseract,
        Err(e) => return Err(e.to_string()),
    });

    let trigger = TesseractTrigger {
        tesseract,
        crop: Crop {
            left_percent: 0.0,
            top_percent: 0.0,
            width_percent: 100.0,
            height_percent: 100.0,
        },
        threshold_filter: ThresholdFilter {
            r: 255,
            g: 255,
            b: 255,
            threshold: 42,
        },
    };

    Hypetrigger::new()
        .set_ffmpeg_exe("ffmpeg".to_string())
        .set_input("test.mp4".to_string())
        .set_fps(2)
        .add_trigger(Box::new(trigger))
        .run()
}
