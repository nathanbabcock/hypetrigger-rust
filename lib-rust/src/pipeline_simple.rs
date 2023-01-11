use crate::debug::debug_photon_image;
use crate::error::{Error, NoneError, Result};
use crate::photon::rgb24_to_rgba32;
use crate::photon::rgba32_to_rgb24;
use crate::photon::ImageTransform;
use crate::photon::{ensure_minimum_size, Crop};
use crate::photon::{ensure_size, ThresholdFilter};
use crate::photon::{ensure_square, rgb_to_photon};
use crate::tensorflow::buffer_to_tensor;
use crate::tensorflow::predict;
use crate::tensorflow::Prediction;
use crate::tensorflow::TENSOR_SIZE;
use crate::threshold::threshold_color_distance_rgba;
use crate::trigger::{Frame, Trigger};
use image::DynamicImage;
use image::ImageError;
use image::RgbImage;
use photon_rs::helpers::dyn_image_from_raw;
use photon_rs::transform::crop;
use photon_rs::transform::padding_uniform;
use photon_rs::PhotonImage;
use photon_rs::Rgb;
use photon_rs::Rgba;
use regex::Regex;
use std::cell::RefCell;
use std::env::current_exe;
use std::fmt::Display;
use std::fs::OpenOptions;
use std::io::stdin;
use std::io::BufReader;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::ChildStderr;
use std::process::ChildStdin;
use std::process::ChildStdout;
use std::sync::mpsc::channel;
use std::sync::mpsc::SendError;
use std::sync::Mutex;
use std::thread;
use std::thread::Scope;
use std::thread::ScopedJoinHandle;
use std::{
    io::{self, BufRead},
    process::{Child, Command, Stdio},
    sync::{
        mpsc::{Receiver, SyncSender},
        Arc,
    },
    thread::JoinHandle,
};
use tensorflow::Graph;
use tensorflow::SavedModelBundle;
use tensorflow::Status;
use tesseract::InitializeError;
use tesseract::Tesseract;

//// Thread Triggers
/// A wrapper around any other Trigger that sends it across a channel to run on
/// a separate thread.
pub struct ThreadTrigger {
    pub trigger: Arc<dyn Trigger + Send + Sync>,
    pub runner_thread: Arc<RunnerThread>,
}

impl Trigger for ThreadTrigger {
    fn on_frame(&self, frame: &Frame) -> Result<()> {
        self.runner_thread
            .tx
            .send(RunnerPayload {
                frame: frame.clone(),
                trigger: self.trigger.clone(),
            })
            .map_err(Error::from_std)
    }
}

impl ThreadTrigger {
    pub fn new<T>(trigger: T, runner_thread: Arc<RunnerThread>) -> Self
    where
        T: Trigger + 'static,
    {
        Self {
            trigger: Arc::new(trigger),
            runner_thread,
        }
    }
}

/// A separate thread that runs one or more ThreadedTriggers, by receiving them
/// over a channel, paired with the frame to process.
pub struct RunnerThread {
    pub tx: SyncSender<RunnerPayload>,
    pub join_handle: JoinHandle<()>,
}

impl RunnerThread {
    /// Prepares a new thread capable of running Triggers, including the
    /// communication channels, spawning the thread itself, and wrapping the
    /// whole struct in an `Arc`.
    pub fn spawn() -> Arc<Self> {
        let (tx, rx) = std::sync::mpsc::sync_channel::<RunnerPayload>(100);
        let join_handle = std::thread::spawn(move || {
            while let Ok(payload) = rx.recv() {
                payload.trigger.on_frame(&payload.frame);
            }
        });
        Arc::new(Self { tx, join_handle })
    }
}

/// Everything a RunnerThread needs to run a ThreadedTrigger
pub struct RunnerPayload {
    frame: Frame,
    trigger: Arc<dyn Trigger>,
}

//// Pipeline
pub struct Hypetrigger {
    // Path the the ffmpeg binary or command to use
    pub ffmpeg_exe: String,

    /// Path to input video (or image) for ffmpeg
    pub input: String,

    /// Framerate to sample the input video at. This can (an should) by much
    /// lower than the input video's native framerate. 2-4 frames per second is
    /// more than sufficient to capture most events.
    pub fps: u64,

    /// List of all callback functions to run on each frame of the video
    pub triggers: Vec<Arc<dyn Trigger>>,
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
    pub fn add_trigger<T>(&mut self, trigger: T) -> &mut Self
    where
        T: Trigger + 'static,
    {
        self.triggers.push(Arc::new(trigger));
        self
    }

    // --- Behavior ---
    /// Spawn ffmpeg, call callbacks on each frame, and block until completion.
    pub fn run(&mut self) -> std::result::Result<(), String> {
        // Init logging
        println!("[hypetrigger] run()");

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

        // Attach to ffmpeg
        self.attach(ffmpeg_stderr, ffmpeg_stdout)?;

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

    pub fn run_async(self) -> Result<(JoinHandle<()>, ChildStdin)> {
        println!("[hypetrigger] run_async()");

        // Spawn FFMPEG command
        let mut ffmpeg_child = self.spawn_ffmpeg_child()?;

        // Separate each stdio channel to use in different places
        let ffmpeg_stderr = ffmpeg_child.stderr.take().ok_or(NoneError)?;
        let ffmpeg_stdout = ffmpeg_child.stdout.take().ok_or(NoneError)?;
        let ffmpeg_stdin = ffmpeg_child.stdin.take().ok_or(NoneError)?;

        // Attach to ffmpeg
        let join_handle = thread::spawn(move || {
            // this blocks (on the inner thread) until the pipeline is done:
            self.attach(ffmpeg_stderr, ffmpeg_stdout)
                .expect("pipeline should complete");
        });

        Ok((join_handle, ffmpeg_stdin))
    }

    pub fn attach(
        &self,
        mut ffmpeg_stderr: ChildStderr,
        mut ffmpeg_stdout: ChildStdout,
    ) -> std::result::Result<(), String> {
        // Enter a new scope that will block until ffmpeg_stderr_thread is done
        thread::scope(|scope| {
            // Spawn a thread to read stderr from ffmpeg
            let (output_size_rx, ffmpeg_stderr_join_handle) =
                match self.spawn_ffmpeg_stderr_thread(&mut ffmpeg_stderr, scope) {
                    Ok(ffmpeg_stderr_thread) => ffmpeg_stderr_thread,
                    Err(e) => {
                        // TODO lost scope:
                        // ffmpeg_child
                        //     .kill()
                        //     .expect("able to stop ffmpeg process if something goes wrong");
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
                frame_num += 1;
            }
            println!("[ffmpeg.out] Finished reading from stdout");
            Ok(())
        })
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
        ScopedJoinHandle<'scope, std::result::Result<(), String>>,
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

/// prints as e.g. `"1:23:45.5"`
pub fn format_seconds(seconds: f64) -> String {
    let mut time_left = seconds;

    let hours = time_left as u64 / 3600;
    time_left -= hours as f64 * 3600.0;

    let minutes = time_left as u64 / 60;
    time_left -= minutes as f64 * 60.0;

    let seconds = time_left as u64;
    time_left -= seconds as f64;

    let milliseconds = (time_left * 1000.0).round() as u64;

    let mut string = "".to_string();
    if hours > 0 {
        string += &format!("{}:", hours);
    }
    if minutes < 10 {
        string += "0";
    }
    string += &format!("{}:", minutes);
    if seconds < 10 {
        string += "0";
    }
    string += &format!("{}", seconds);
    if milliseconds > 0 {
        string += &format!(".{}", milliseconds);
    }
    string
}
