use crate::{logging::LoggingConfig, runner::RunnerContext};
use image::DynamicImage;
use std::{
    io::stdin,
    sync::{Arc, RwLock},
};

#[derive(Clone)]
pub struct DebuggerStep {
    /// The context of this step, including:
    /// - The config of the Job that is invoking this run
    /// - Frame number of the input media
    /// - The specific Trigger currently being run on that frame
    pub context: RunnerContext,

    /// An explanation of what this step of the pipeline is doing
    ///
    /// For example:
    /// - "Receiving raw image from ffmpeg"
    /// - "Applying threshold filter to image"
    /// - "Running Tesseract OCR to detect text"
    pub description: String,

    /// An optional image representation of the current step of the pipeline
    /// Can be written to disk for debugging purposes
    /// In the case of non-image steps (e.g. text or regex parsing), this will be None
    pub image: Option<DynamicImage>,
}
pub enum DebuggerState {
    /// Blocking, waiting for user commands on stdin
    Paused,

    /// Skipping steps until the next input frame
    JumpingToNextFrame,

    /// Skipping steps until reaching a different Trigger invocation
    JumpingToNextTrigger,

    /// Freely running until manually paused by `Debugger::pause()`
    Resumed,
}

pub struct Debugger {
    pub state: DebuggerState,
    pub cur_step: Option<DebuggerStep>,
    pub prev_step: Option<DebuggerStep>,

    /// whether to log to stdout, to save log file, to save images
    pub _debugger_config: LoggingConfig,
}

impl Debugger {
    /// Pauses the debugger (at the next step/breakpoint), then blocks and waits
    /// for user input
    pub fn pause(this: Arc<RwLock<Self>>) {
        this.write().unwrap().state = DebuggerState::Paused;
    }

    /// Pauses the debugger (at the next step/breakpoint), then blocks and waits
    /// for user input
    pub fn resume(this: Arc<RwLock<Self>>) {
        let mut debugger = this.write().unwrap();
        debugger.state = DebuggerState::Resumed;
        debugger.cur_step = None;
        debugger.prev_step = None;
    }

    /// Attach an entry point for the debugger to (potentially) pause and inspect
    /// the current state of execution.
    ///
    /// If the debugger is not active, this function is skipped over and instantly returns.
    pub fn register_step(this: Arc<RwLock<Self>>, step: DebuggerStep) {
        let this_clone = this.clone();
        let debugger = this.read().unwrap();
        match debugger.state {
            DebuggerState::Resumed => return,
            _ => {}
        }

        // TODO optionally write to log file

        Debugger::step_stdout(&step);
        Debugger::step_stdin(this_clone, &step);
    }

    pub fn step_stdout(step: &DebuggerStep) {
        println!("[debugger] execution paused.");
        println!(" - input path: {}", step.context.config.inputPath);
        println!(" - frame number: {}", step.context.frame_num);
        println!(" - timestamp: {}", step.context.get_timestamp());
        println!(
            " - trigger type: {}",
            step.context.trigger.get_runner_type()
        );
        println!(" - current step: {}", step.description);

        if let Some(image) = &step.image {
            Debugger::handle_step_image(image);
        }

        // TODO: make these more compact, with colors, and cleared afterwards
    }

    /// Save a temporary image to file, and log the path and dimensions to stdout
    pub fn handle_step_image(image: &DynamicImage) {
        println!(" - current image ({}x{})", image.width(), image.height());
        // let dyn_image = dyn_image_from_raw(&image);
        let path = "current-frame.temp.bmp"; // todo create temp folder
        image
            .save(path)
            .unwrap_or_else(|e| eprintln!("failed to save image: {:?}", e));
        // open::that(path).unwrap_or_else(|e| eprintln!("failed to open image: {:?}", e)); // todo; only open the first time
        println!(" - image path: {}", path);
    }

    /// Blocks while waiting for the user's command
    pub fn step_stdin(_this: Arc<RwLock<Self>>, step: &DebuggerStep) {
        println!("[debugger] press enter to continue.");
        stdin().read_line(&mut String::new()).unwrap();
    }

    pub fn handle_command(_this: Arc<RwLock<Self>>, _command: &str) {
        todo!("");
    }

    /// Clears the last few lines of console output
    pub fn clear_step(_this: Arc<RwLock<Self>>, _step: DebuggerStep) {
        todo!("");
    }
}

impl Default for Debugger {
    fn default() -> Self {
        Self {
            state: DebuggerState::Resumed,
            cur_step: None,
            prev_step: None,
            _debugger_config: LoggingConfig::default(),
        }
    }
}
