use crate::{config::HypetriggerConfig, runner::RunnerContext, trigger::Trigger};
use image::DynamicImage;
use std::{
    env::{current_dir, current_exe},
    fs::{self, OpenOptions},
    io::{self, stdin, Write},
    path::Path,
    sync::{Arc, RwLock},
};

/// convenient alias for passing a debugger between threads
pub type DebuggerRef = Arc<RwLock<Debugger>>;

#[derive(Clone)]
pub struct DebuggerStep {
    /// The context of this step, including:
    /// - The config of the Job that is invoking this run
    /// - Frame number of the input media
    /// - The specific Trigger currently being run on that frame
    // pub context: RunnerContext,

    /// The config of the Job that is invoking this run
    pub config: Arc<HypetriggerConfig>,

    /// The specific Trigger that is currently being run
    pub trigger: Arc<dyn Trigger>,

    /// Monotonically inreasing by 1, starting from 0. Does not correspond
    /// directly to the frame number of the source video, because it is
    /// (typically) sampled at a lower framerate.
    pub frame_num: u64,

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
    // pub cur_step: Option<DebuggerStep>,
    // pub prev_step: Option<DebuggerStep>,
    pub log_to_disk: bool,
    pub log_file: String,

    /// Whether the log file has been created and/or cleared from the previous run
    log_initialized: bool,
}

impl Debugger {
    /// Pauses the debugger (at the next step/breakpoint), then blocks and waits
    /// for user input
    pub fn pause(this: DebuggerRef) {
        this.write().unwrap().state = DebuggerState::Paused;
    }

    /// Pauses the debugger (at the next step/breakpoint), then blocks and waits
    /// for user input
    pub fn resume(this: DebuggerRef) {
        let mut debugger = this.write().unwrap();
        debugger.state = DebuggerState::Resumed;
        // debugger.cur_step = None;
        // debugger.prev_step = None;
    }

    /// Writes a line to the log file on disk (if enabled)
    pub fn log(&self, message: &str) {
        if !self.log_to_disk {
            return;
        }

        // TODO should this be kept open? or opened/closed each time?
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_file.clone())
            .unwrap_or_else(|e| {
                eprintln!("Couldn't open log file: {}", e);
                panic!("unrecoverable");
            });

        // // temporary
        // println!("{}", self.log_file);
        // println!("[debugger] press enter to continue.");
        // stdin().read_line(&mut String::new()).unwrap();

        if let Err(e) = writeln!(file, "{}", message) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }

    /// Delete old log file, if present.
    pub fn init_log(mut self) -> Self {
        match self.log_initialized {
            true => return self,
            false => self.log_initialized = true,
        }
        // Regardless of what happens, mark the log as initialized.
        // This method is intended as a singleton which runs only once.
        // In other languages, it would happen in the constructor.
        // In Rust, this struct might be created many different ways
        // (builder pattern, Default trait, raw struct instantiation, etc).
        // So instead, we use a flag to ensure it only runs once.

        // Check for a previous log file
        if Path::new(self.log_file.as_str()).exists() {
            println!("[debugger] Deleting previous log file: {}", self.log_file);

            // Handle errors internally
            match fs::remove_file(self.log_file.clone()) {
                Ok(_) => println!("[debugger] Deleted old log file"),
                Err(e) => eprintln!("[debugger] Couldn't delete old log file: {}", e),
            };
        }
        println!("[debugger] Log file initialized: {}", self.log_file);
        self
    }

    /// Attach an entry point for the debugger to (potentially) pause and inspect
    /// the current state of execution.
    ///
    /// If the debugger is not active, this function is skipped over and instantly returns.
    pub fn register_step(this: DebuggerRef, step: DebuggerStep) {
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
        println!(" - input path: {}", step.config.inputPath);
        println!(" - frame number: {}", step.frame_num);
        // println!(" - timestamp: {}", step.get_timestamp()); // TODO lost in time
        println!(" - trigger type: {}", step.trigger.get_runner_type());
        println!(" - current step: {}", step.description);

        if let Some(image) = &step.image {
            Debugger::handle_step_image(image);
        }

        // TODO: make these more compact, with colors, and cleared afterwards
    }

    /// Save a temporary image to file, and log the path and dimensions to stdout
    pub fn handle_step_image(image: &DynamicImage) {
        println!(" - current image ({}x{})", image.width(), image.height());
        let dir = current_exe().unwrap();
        let path_buf = dir.parent().unwrap().join("current-frame.tmp.bmp");
        let path = path_buf.as_os_str().to_str().unwrap();
        // TODO make this configurable at a higher scope
        // TODO create temp folder
        image
            .save(path)
            .unwrap_or_else(|e| eprintln!("failed to save image: {:?}", e));
        println!(" - image path: {}", path);
    }

    /// Blocks while waiting for the user's command
    pub fn step_stdin(_this: DebuggerRef, step: &DebuggerStep) {
        println!("[debugger] press enter to continue.");
        stdin().read_line(&mut String::new()).unwrap();
    }

    pub fn handle_command(_this: DebuggerRef, _command: &str) {
        todo!("");
    }

    /// Clears the last few lines of console output
    pub fn clear_step(_this: DebuggerRef, _step: DebuggerStep) {
        todo!("");
    }

    /// Create a new default Debugger instance AND initialize the log file
    pub fn new() -> Self {
        let mut debugger = Self {
            state: DebuggerState::Resumed,
            // cur_step: None,
            // prev_step: None,
            log_to_disk: true,
            log_file: current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .join("hypetrigger.tmp.log")
                .as_os_str()
                .to_str()
                .unwrap()
                .to_string(),
            log_initialized: false,
            // TODO: log directory
            // TODO: image filename
        };

        // Initialize the log file. If this struct is created some other way,
        // this will need to be called manually.
        debugger.init_log()
    }
}

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}
