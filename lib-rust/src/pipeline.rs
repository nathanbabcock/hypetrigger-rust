use crate::{
    error::{Error, NoneError, Result},
    trigger::{Frame, Trigger},
    util::{command_to_string, parse_ffmpeg_output_size},
};
use image::RgbImage;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::{
    io::BufRead,
    process::{ChildStdin, ChildStdout, Command, Stdio},
    thread::JoinHandle,
};
use std::{
    io::BufReader,
    process::ChildStderr,
    sync::{
        mpsc::{channel, Receiver},
        Arc,
    },
    thread::{self, Scope, ScopedJoinHandle},
};
use std::{io::Read, process::Child};

pub type HypetriggerOnCompleteCallback = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct Hypetrigger {
    // Path the the ffmpeg binary or command to use
    pub ffmpeg_exe: String,

    /// Path to input video (or image) for ffmpeg. Corresponds to ffmpeg `-i` arg.
    pub input: String,

    /// Less commonly used, indicates the video format of the input, if it can't
    /// be inferred from the file extension. Corresponds to ffmpeg `-f` arg.
    ///
    /// Use cases:
    /// - generating test source (`-f lavfi -i testsrc`)
    /// - certain methods of screen capture (`-f gdigrab`).
    pub input_format: Option<String>,

    /// Framerate to sample the input video at. This can (an should) by much
    /// lower than the input video's native framerate. 2-4 frames per second is
    /// more than sufficient to capture most events.
    pub fps: u64,

    /// List of all callback functions to run on each frame of the video
    pub triggers: Vec<Arc<dyn Trigger>>,

    /// Callback when the video is finished processing. Particularly useful in
    /// combination with `run_async`.
    pub on_complete_callback: Option<HypetriggerOnCompleteCallback>,
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
            input_format: None,
            fps: 2,
            triggers: vec![],
            on_complete_callback: None,
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

    /// Setter for the input vformat for ffmpeg
    pub fn set_input_format(&mut self, input_format: &str) -> &mut Self {
        self.input_format = Some(input_format.to_string());
        self
    }

    /// Alias for `set_input_format("lavfi")` and `set_input(FFMPEG_TEST_INPUT)`
    pub fn test_input(&mut self) -> &mut Self {
        self.set_input_format("lavfi")
            .set_input(FFMPEG_TEST_INPUT.to_string())
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

    /// Add a dynamic array slice of Triggers
    pub fn add_triggers(&mut self, triggers: &[Arc<dyn Trigger>]) -> &mut Self {
        for trigger in triggers {
            self.triggers.push(trigger.clone());
        }
        self
    }

    /// Call the given function when the input finishes processing
    pub fn on_complete<T>(&mut self, callback: T) -> &mut Self
    where
        T: Fn() + Send + Sync + 'static,
    {
        self.on_complete_callback = Some(Arc::new(callback));
        self
    }

    // --- Behavior ---
    /// Spawn ffmpeg, call callbacks on each frame, and block until completion.
    pub fn run(&mut self) -> Result<()> {
        println!("[hypetrigger] run()");

        // Spawn FFMPEG command
        let mut ffmpeg_child = self.spawn_ffmpeg_child()?;
        let ffmpeg_stderr = ffmpeg_child.stderr.take().ok_or(NoneError)?;
        let ffmpeg_stdout = ffmpeg_child.stdout.take().ok_or(NoneError)?;

        // Attach to ffmpeg
        self.attach(ffmpeg_stderr, ffmpeg_stdout)?;

        // Block until ffmpeg finishes
        let ffmpeg_exit_status = ffmpeg_child.wait()?;
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

    /// Set up the watchers for the ffmpeg process on stdout and stderr. Stderr
    /// will run in a separate scoped thread, while stdout will run on the
    /// current thread, and then block until completion.
    pub fn attach(
        &self,
        mut ffmpeg_stderr: ChildStderr,
        mut ffmpeg_stdout: ChildStdout,
    ) -> Result<()> {
        // Enter a new scope that will block until ffmpeg_stderr_thread is done
        thread::scope(|scope| {
            // Spawn a thread to read stderr from ffmpeg
            let (output_size_rx, _ffmpeg_stderr_join_handle) =
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
                    if let Err(e) = trigger.on_frame(&frame) {
                        eprintln!("Error in trigger: {}", e);
                    }
                }
                frame_num += 1;
            }
            println!("[ffmpeg.out] Finished reading from stdout");
            Ok(())
        }).map_err(Error::from)
    }

    pub fn spawn_ffmpeg_child(&self) -> Result<Child> {
        let mut cmd = Command::new(self.ffmpeg_exe.as_str());
        cmd.arg("-hwaccel").arg("auto");
        if let Some(input_format) = &self.input_format {
            cmd.arg("-f").arg(input_format);
        }
        cmd.arg("-i")
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

        cmd.spawn().map_err(Error::from)
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
    ) -> Result<(Receiver<(u32, u32)>, FfmpegStderrJoinHandle<'scope>)> {
        let (output_size_tx, output_size_rx) = channel::<(u32, u32)>();
        let thread_body = move || {
            let reader = BufReader::new(ffmpeg_stderr);
            let mut current_section = "";
            let mut output_size: Option<(u32, u32)> = None;
            for line in reader.lines() {
                let text = match line {
                    Err(e) => {
                        eprintln!("[ffmpeg.err] Error reading ffmpeg stderr: {}", e);
                        eprintln!("[ffmpeg.err] Attempting to continue reading next line.");
                        continue;
                    }
                    Ok(text) => text,
                };

                // Parse for output size if not already found
                if output_size.is_none() {
                    if text.starts_with("Output #") {
                        current_section = "Output"; // stringly-typed rather than enum for convenience
                    } else if current_section == "Output" {
                        if let Some(size) = parse_ffmpeg_output_size(text.as_str()) {
                            output_size = Some(size); // remember this, so we don't check for it anymore
                            output_size_tx.send(size).map_err(|e| e.to_string())?;
                        }
                    }
                }

                // Regular callback on every line of stderr
                println!("[ffmpeg.err] {}", text.trim_end());
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

pub type FfmpegStderrJoinHandle<'scope> =
    ScopedJoinHandle<'scope, core::result::Result<(), String>>;

/// Used with the ffmpeg `-i` argument, or with `.input()` in the Hypetrigger API.
/// <https://www.bogotobogo.com/FFMpeg/ffmpeg_video_test_patterns_src.php>
pub const FFMPEG_TEST_INPUT: &str = "testsrc=duration=10:size=1280x720:rate=30";

/// Sends a `q` to the ffmpeg process over stdin, which tells it gracefully exit.
/// You could also call `kill()` on the `Child` process instance of ffmpeg to stop it
/// more abruptly. You can obtain the `stdin` handle from the return value of `run_async()`.
pub fn stop_ffmpeg(stdin: &mut ChildStdin) -> Result<()> {
    stdin.write_all(b"q\n").map_err(Error::from)
}
