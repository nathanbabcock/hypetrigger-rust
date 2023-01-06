use crate::{config::HypetriggerConfig, runner::RunnerContext, trigger::Trigger};
use image::{DynamicImage, RgbImage};
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
    pub image: Option<RgbImage>,
}
pub enum DebuggerState {
    /// Blocking, waiting for user commands on stdin
    Paused,

    /// Skipping steps until the next input frame
    // JumpingToNextFrame,

    /// Skipping steps until reaching a different Trigger invocation
    // JumpingToNextTrigger,

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
    pub fn pause(this: &DebuggerRef) {
        this.write().unwrap().state = DebuggerState::Paused;
    }

    /// Disable the debugger and continue execution without breakpoints
    pub fn resume(this: DebuggerRef) {
        let mut debugger = this.write().unwrap();
        debugger.state = DebuggerState::Resumed;
        println!("[debugger] Resuming...");
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
        Debugger::step_stdin(this_clone);
    }

    pub fn step_stdout(step: &DebuggerStep) {
        let input_path = Path::new(step.config.inputPath.as_str());
        let input_filename = input_path.file_name().unwrap().to_str().unwrap();
        println!(
            "[debugger] execution paused. {}, frame {}",
            input_filename, step.frame_num
        );
        println!(" > Current step: {}", step.description);

        // Handle image
        let image_path = match &step.image {
            Some(image) => {
                let dir = current_exe().unwrap();
                let image_path_buf = dir.parent().unwrap().join("current-frame.tmp.bmp");
                let image_path_str = image_path_buf.as_os_str().to_str().unwrap();
                image
                    .save(image_path_str)
                    .unwrap_or_else(|e| eprintln!("failed to save image: {:?}", e));
                image_path_str.to_string()
            }
            None => "(none)".to_string(),
        };
        println!(" > Preview: {}", image_path);
        // TODO: make these more compact, with colors, and cleared afterwards
    }

    /// Blocks while waiting for the user's command
    pub fn step_stdin(this: DebuggerRef) {
        println!("[debugger] Enter command to continue: step (s), resume (r)");
        let mut command: String = "".into();
        stdin().read_line(&mut command).unwrap();
        match command.trim() {
            "step" | "s" => Debugger::step(this),
            // "next_trigger" | "nt" => Debugger::next_trigger(this),
            // "next_frame" | "nf" => Debugger::next_frame(this),
            "resume" | "r" => Debugger::resume(this),
            _ => {
                println!("[debugger] Unrecognized command: {}", command);
                Debugger::step_stdin(this);
            }
        }
    }

    /// Continue execution until the next breakpoint
    pub fn step(this: DebuggerRef) {
        // Debugger is already paused, so it will automatically stop at the next
        // available step breakpoint
        Debugger::clear_step(this);
    }

    // /// Skip all breakpoints until we begin a new Trigger execution
    // pub fn next_trigger(this: DebuggerRef) {
    //     let mut debugger = this.write().unwrap();
    //     debugger.state = DebuggerState::JumpingToNextTrigger;
    // }

    // /// Skip all breakpoints until we reach the next frame from the input media
    // pub fn next_frame(this: DebuggerRef) {
    //     let mut debugger = this.write().unwrap();
    //     debugger.state = DebuggerState::JumpingToNextFrame;
    // }

    /// Clears the last few lines of console output
    pub fn clear_step(this: DebuggerRef) {
        // seems like this might be harder than intially expected...
        // skip for now!
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
