use image::RgbImage;
use photon_rs::PhotonImage;
use regex::Regex;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::ChildStderr;
use std::sync::mpsc::channel;
use std::thread;
use std::thread::Scope;
use std::thread::ScopedJoinHandle;
use std::{
    io::{self, BufRead},
    process::{Child, Command, Stdio},
    sync::{
        mpsc::{Receiver, SyncSender},
        Arc, Mutex,
    },
    thread::JoinHandle,
};
use tesseract::Tesseract;

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

/// Represents a single frame of the input, including the raw image pixels as
/// well as the time it appears in the input (frame_num and/or timestamp)
#[derive(Clone)]
pub struct Frame {
    pub image: RgbImage,
    pub frame_num: u64,
    pub timestamp: f64,
}

//// Triggers
pub trait Trigger {
    fn on_frame(&self, frame: &Frame) -> Result<(), String>;

    /// Convert this Trigger into a ThreadTrigger, running on a separate thread.
    fn run_on_thread(self, runner_thread: RunnerThread) -> ThreadTrigger
    where
        Self: Sized + Send + Sync + 'static,
    {
        ThreadTrigger {
            trigger: Arc::new(self),
            runner_thread,
        }
    }
}

//// Tesseract
pub struct TesseractTrigger {
    tesseract: Mutex<Tesseract>,
    crop: Crop,
    threshold_filter: ThresholdFilter,
}

impl Trigger for TesseractTrigger {
    fn on_frame(&self, frame: &Frame) -> Result<(), String> {
        Err("not implemented".to_string())
    }
}

//// "Threaded Triggers"

/// A wrapper around any other Trigger that sends it across a channel to run on
/// a separate thread.
pub struct ThreadTrigger {
    trigger: Arc<dyn Trigger + Send + Sync>,
    runner_thread: RunnerThread,
}

impl Trigger for ThreadTrigger {
    fn on_frame(&self, frame: &Frame) -> Result<(), String> {
        match self.runner_thread.tx.send(RunnerPayload {
            frame: frame.clone(),
            trigger: self.trigger.clone(),
        }) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// A separate thread that runs one or more ThreadedTriggers, by receiving them
/// over a channel, paired with the frame to process.
pub struct RunnerThread {
    // rx: Receiver<RunnerPayload>,
    pub tx: SyncSender<RunnerPayload>,
    pub join_handle: JoinHandle<()>,
}

impl RunnerThread {
    pub fn spawn(buffer_size: usize) -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<RunnerPayload>(buffer_size);
        let join_handle = std::thread::spawn(move || {
            while let Ok(payload) = rx.recv() {
                payload.trigger.on_frame(&payload.frame);
            }
        });
        Self { tx, join_handle }
    }
}

/// Everything a RunnerThread needs to run a ThreadedTrigger
pub struct RunnerPayload {
    frame: Frame,
    trigger: Arc<dyn Trigger + Send + Sync>,
}

//// Pipeline
pub struct Hypetrigger {
    // Path the the ffmpeg binary or command to use
    pub ffmpeg_exe: String,

    // Path (including filename) to save a log file
    pub log_file: Option<PathBuf>,

    /// Path to input video (or image) for ffmpeg
    pub input: String,

    /// Framerate to sample the input video at. This can (an should) by much
    /// lower than the input video's native framerate. 2-4 frames per second is
    /// more than sufficient to capture most events.
    pub fps: u64,

    /// List of all callback functions to run on each frame of the video
    pub triggers: Vec<Box<dyn Trigger>>,
}

impl Default for Hypetrigger {
    fn default() -> Self {
        Self::new()
    }
}

impl Hypetrigger {
    // --- Constructor ---
    pub fn new() -> Self {
        Self {
            ffmpeg_exe: "ffmpeg".to_string(),
            log_file: Some(PathBuf::from("hypetrigger.log")),
            input: "".to_string(),
            fps: 2,
            triggers: vec![],
        }
    }

    // --- Getters and setters ---
    /// Setter for the ffmpeg binary or command to use
    pub fn set_ffmpeg_exe(&mut self, ffmpeg_exe: String) -> &mut Self {
        self.ffmpeg_exe = ffmpeg_exe;
        self
    }

    /// Specify where to save log file to disk, or `None` to disable logging
    pub fn set_log_file<P>(&mut self, log_file: Option<P>) -> &mut Self
    where
        P: Into<PathBuf>,
    {
        self.log_file = log_file.map(|path_ref| path_ref.into());
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

    // --- Behavior ---
    /// Spawn ffmpeg, call callbacks on each frame, and block until completion.
    pub fn run(&mut self) -> Result<(), String> {
        // Init logging
        println!("[hypetrigger] run()");
        self.init_log();

        // Spawn FFMPEG command
        let mut ffmpeg_child = match self.spawn_ffmpeg_child() {
            Ok(ffmpeg_child) => ffmpeg_child,
            Err(e) => return Err(e.to_string()),
        };

        // Extract each stdio channel to use in different places
        let mut ffmpeg_stderr = match ffmpeg_child.stderr.take() {
            Some(ffmpeg_stderr) => ffmpeg_stderr,
            None => return Err("no stderr".to_string()),
        };
        let mut ffmpeg_stdout = match ffmpeg_child.stdout.take() {
            Some(ffmpeg_stdout) => ffmpeg_stdout,
            None => return Err("no stdout".to_string()),
        };
        let ffmpeg_stdin = match ffmpeg_child.stdin.take() {
            Some(ffmpeg_stdin) => ffmpeg_stdin,
            None => return Err("no stdin".to_string()),
        };

        // Enter a new scope that will block until ffmpeg_stderr_thread is done
        thread::scope(|scope| {
            // Spawn a thread to read stderr from ffmpeg
            let (output_size_rx, ffmpeg_stderr_join_handle) =
                match self.spawn_ffmpeg_stderr_thread(&mut ffmpeg_stderr, scope) {
                    Ok(ffmpeg_stderr_thread) => ffmpeg_stderr_thread,
                    Err(e) => {
                        ffmpeg_child
                            .kill()
                            .expect("able to stop ffmpeg process if something goes wrong");
                        return Err(e.to_string());
                    }
                };

            // Block on each line of ffmpeg stderr until receiving the output size
            let (output_width, output_height) = output_size_rx.recv().map_err(|_| {
            "ffmpeg exited before sending output size. This is likely due to an invalid input file.".to_string()
          })?;
            println!(
                "[ffmpeg] Parsed output size from logs: {}x{}",
                output_width, output_height
            );

            // Initialize a buffer
            const CHANNELS: u32 = 3; // matches in the `-f rgb24` flag to ffmpeg
            let buf_size = (output_width * output_height * CHANNELS) as usize;
            let mut buffer = vec![0_u8; buf_size];
            println!("[ffmpeg.stdout] Allocated buffer of size {}", buf_size);

            // Read from stdout on the current thread, invoking Triggers each frame
            let mut frame_num = 0;
            while ffmpeg_stdout.read_exact(&mut buffer).is_ok() {
                let image = match RgbImage::from_vec(output_width, output_height, buffer.clone()) {
                    Some(image) => image,
                    None => {
                        return Err(
                            "unable to convert vec to imagebuffer (size mismatch)".to_string()
                        )
                    }
                };
                let frame = Frame {
                    image,
                    frame_num,
                    timestamp: frame_num as f64 / self.fps as f64,
                };
                for trigger in &self.triggers {
                    trigger.on_frame(&frame);
                }
            }
            println!("[ffmpeg.out] Finished reading from stdout");
            Ok(())
        })?;

        // Block until ffmpeg finishes
        let ffmpeg_exit_status = match ffmpeg_child.wait() {
            Ok(ffmpeg_exit_status) => ffmpeg_exit_status,
            Err(e) => return Err(e.to_string()),
        };
        println!(
            "[ffmpeg] ffmpeg command exited with status {}",
            ffmpeg_exit_status
        );

        Ok(())
    }

    /// Write a message to the log file on disk. If `log_file` is `None`, this is a no-op.
    pub fn log<S>(&self, message: &str) -> io::Result<()> {
        match &self.log_file {
            Some(path) => {
                let mut file = OpenOptions::new().create(true).append(true).open(path)?;
                writeln!(file, "{}", message)
            }
            None => Ok(()),
        }
    }

    /// Initialize the log file on disk. If `log_file` is `None`, this is a no-op.
    pub fn init_log(&mut self) -> &mut Self {
        let result = match &self.log_file {
            Some(path) => match OpenOptions::new().create(true).write(true).open(path) {
                Ok(mut file) => writeln!(file, ""),
                Err(e) => Err(e),
            },
            None => {
                println!("[logger] Log file is set to None.");
                Ok(())
            }
        };
        if let Err(e) = result {
            eprintln!("[logger] Failed to initialize log file: {}", e);
            eprintln!("[logger] Continuing without logging.");
            self.set_log_file::<PathBuf>(None); // ?? turbofish is necessary here?
        };
        self
    }

    pub fn spawn_ffmpeg_child(&self) -> io::Result<Child> {
        let mut cmd = Command::new(self.ffmpeg_exe.as_str());
        cmd.arg("-hwaccel")
            .arg("auto")
            .arg("-i")
            .arg(self.input.as_str())
            .arg("-filter:v")
            .arg(format!("fps={}", self.fps))
            .arg("-vsync")
            .arg("drop")
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgb24")
            .arg("-an")
            .arg("-y")
            .arg("pipe:1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(0x08000000); // this seems Windows-only. Is there a cross-platform solution?

        // Debug command
        println!("[debug] ffmpeg command appears below:");
        println!("{}", command_to_string(&cmd));

        cmd.spawn()
    }

    /// Spawns a thread to handle reading the stderr channel from ffmpeg.
    ///
    /// After first spawning, we read the metadata/prelude of the ffmpeg job in
    /// order to determine the width and height of the output frames. That's
    /// sent back to the main thread via a channel. After the metadata is
    /// received, the channel closes, while the stderr handler thread continues
    /// to run in the background. It automatically stops after ffmpeg exits.
    pub fn spawn_ffmpeg_stderr_thread<'scope>(
        &'scope self,
        ffmpeg_stderr: &'scope mut ChildStderr,
        scope: &'scope Scope<'scope, '_>, // scope scope scope scope wheeee
    ) -> io::Result<(
        Receiver<(u32, u32)>,
        ScopedJoinHandle<'scope, Result<(), String>>,
    )> {
        let (output_size_tx, output_size_rx) = channel::<(u32, u32)>();
        let thread_body = move || {
            let mut reader = BufReader::new(ffmpeg_stderr);
            let mut line = String::new();
            let mut current_section = "";
            let mut output_size: Option<(u32, u32)> = None;
            loop {
                // Rust docs claim this isn't necessary, but the buffer
                // never gets cleared!
                line.clear();

                match reader.read_line(&mut line) {
                    Ok(0) => {
                        break; // (EOF)
                    }
                    Ok(_) => {
                        // Parse for output size if not already found
                        if output_size.is_none() {
                            if line.starts_with("Output #") {
                                current_section = "Output"; // stringly-typed rather than enum for convenience
                            } else if current_section == "Output" {
                                if let Some(size) = parse_ffmpeg_output_size(line.as_str()) {
                                    output_size = Some(size); // remember this, so we don't check for it anymore
                                    output_size_tx.send(size).map_err(|e| e.to_string())?;
                                }
                            }
                        }

                        // Regular callback on every line of stderr
                        println!("[ffmpeg.err] {}", line.trim_end());
                        // TODO: switch this to `self.on_ffmpeg_stderr`
                        // callback (possible in a scoped thread)
                    }
                    Err(e) => {
                        eprintln!("[ffmpeg.err] Error reading ffmpeg stderr: {}", e);
                        eprintln!("[ffmpeg.err] Attempting to continue reading next line.");
                    }
                }
            }

            println!("[ffmpeg.err] ffmpeg stderr thread exiting");
            Ok(())
        };

        let join_handle = thread::Builder::new()
            .name("ffmpeg_stderr".to_string())
            .spawn_scoped(scope, thread_body)?;

        Ok((output_size_rx, join_handle))
    }
}

//// Utilities

/// Convert a Command to a string that can be run in a shell (for debug
/// purposes).
///
/// It's tailored to the `ffmpeg` command, such that it pairs up groups of
/// arguments prefixed with dashes with their corresponding values (e.g. `-i`
/// and `input.mp4`), and splits them onto multiple (escaped) lines for
/// readibility.
pub fn command_to_string(cmd: &Command) -> String {
    let mut command_string = String::new();
    command_string.push_str(cmd.get_program().to_str().unwrap());

    for arg in cmd.get_args() {
        let arg_str = arg.to_str().unwrap();
        command_string.push(' ');
        if arg_str.starts_with('-') {
            command_string.push_str("\\\n\t");
            command_string.push_str(arg_str);
        } else {
            command_string.push_str(format!("{:?}", arg_str).as_str());
        }
    }

    command_string
}

/// Parses a line of ffmpeg stderr output, looking for the video size.
/// We're looking for a line like this:
///
/// ```
///   Stream #0:0(und): Video: rawvideo (RGB[24] / 0x18424752), rgb24(pc, bt709, progressive), 1920x1080 [SAR 1:1 DAR 16:9], q=2-31, 99532 kb/s, 2 fps, 2 tbn (default)
/// ```
pub fn parse_ffmpeg_output_size(text: &str) -> Option<(u32, u32)> {
    lazy_static! {
        static ref REGEX_SIZE: Regex = Regex::new(r"  Stream .* Video: .* (\d+)x(\d+),? ").unwrap();
    }

    match REGEX_SIZE.captures(text) {
        Some(capture) => {
            let width = capture.get(1).unwrap().as_str().parse::<u32>().unwrap();
            let height = capture.get(2).unwrap().as_str().parse::<u32>().unwrap();
            Some((width, height))
        }
        None => None,
    }
}
