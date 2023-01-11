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

//// Pipeline

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
