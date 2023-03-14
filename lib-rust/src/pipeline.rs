use crate::{
    error::{Error, Result},
    trigger::{Frame, Trigger},
};
use ffmpeg_sidecar::{command::FfmpegCommand, event::FfmpegEvent};
use image::RgbImage;
use std::io::Write;
use std::{process::ChildStdin, thread::JoinHandle};
use std::{
    sync::Arc,
    thread::{self},
};

#[derive(Clone)]
pub struct Hypetrigger {
    /// Path the the ffmpeg binary or command to use
    pub ffmpeg_exe: String,

    /// Print all FFmpeg log output to stderr
    pub verbose: bool,

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
            verbose: false,
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

    /// Enable or disable verbose logging
    pub fn set_verbose(&mut self, verbose: bool) -> &mut Self {
        self.verbose = verbose;
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

    /// Spawn the inner FFmpeg command. This is a lower-level function that
    /// doesn't need to be used directly. It's equivalent to `FFmpegCommand`
    /// from `ffmpeg-sidecar` with some preset arguments and configuration.
    pub fn ffmpeg_command(&self) -> FfmpegCommand {
        let mut cmd = FfmpegCommand::new_with_path(self.ffmpeg_exe.as_str());
        cmd.hwaccel("auto");
        if let Some(input_format) = &self.input_format {
            cmd.format(input_format);
        }
        cmd.input(self.input.as_str())
            .args(["-filter:v", &format!("fps={}", self.fps)])
            .args(["-vsync", "drop"])
            .no_audio() // -an
            .overwrite() // -y
            .rawvideo();
        cmd
    }

    /// A lower-level function handles both running triggers on each output
    /// frame of FFmpeg, as well as logging when appropriate.
    pub fn handle_triggers(&self, event: FfmpegEvent) -> Result<()> {
        match event {
            FfmpegEvent::OutputFrame(frame) => {
                let image = RgbImage::from_vec(frame.width, frame.height, frame.data)
                    .ok_or("Failed to get image from frame")?;
                let frame = Frame {
                    image,
                    frame_num: frame.frame_num as u64,
                    timestamp: frame.timestamp as f64,
                };
                self.triggers
                    .iter()
                    .map(|trigger| trigger.on_frame(&frame))
                    .all(|r| r.is_ok())
                    .then_some(())
                    .ok_or(format!(
                        "One or more triggers failed to run on frame {}",
                        frame.frame_num
                    ))?;
            }
            FfmpegEvent::LogError(msg) | FfmpegEvent::Error(msg) => {
                eprintln!("[ffmpeg] {}", msg)
            }
            e if self.verbose => println!("[ffmpeg] {:?}", e),
            _ => {}
        }
        Ok(())
    }

    /// Spawn ffmpeg, call callbacks on each frame, and block until completion.
    pub fn run(&mut self) -> Result<()> {
        self.ffmpeg_command()
            .spawn()?
            .iter()?
            .for_each(|event| self.handle_triggers(event).unwrap_or(()));
        Ok(())
    }

    /// Same as calling `run` on a separate thread, returning both the thread's
    /// `JoinHandle` as well as the `ChildStdin` that can be used to stop
    /// the pipeline early.
    ///
    /// See also: `stop_ffmpeg(child: ChildStdin)`
    pub fn run_async(self) -> Result<(JoinHandle<()>, ChildStdin)> {
        let mut child = self.ffmpeg_command().spawn()?;
        let ffmpeg_stdin = child.take_stdin().ok_or("Failed to get stdin")?;
        let iter = child.iter()?;
        let join_handle = thread::spawn(move || {
            iter.for_each(|event| self.handle_triggers(event).unwrap_or(()));
        });
        Ok((join_handle, ffmpeg_stdin))
    }
}

pub type HypetriggerOnCompleteCallback = Arc<dyn Fn() + Send + Sync>;

/// Used with the ffmpeg `-i` argument, or with `.input()` in the Hypetrigger API.
/// <https://www.bogotobogo.com/FFMpeg/ffmpeg_video_test_patterns_src.php>
pub const FFMPEG_TEST_INPUT: &str = "testsrc=duration=10:size=1280x720:rate=30";

/// Sends a `q` to the ffmpeg process over stdin, which tells it gracefully exit.
/// You could also call `kill()` on the `Child` process instance of ffmpeg to stop it
/// more abruptly. You can obtain the `stdin` handle from the return value of `run_async()`.
pub fn stop_ffmpeg(stdin: &mut ChildStdin) -> Result<()> {
    stdin.write_all(b"q\n").map_err(Error::from)
}
