use std::process::Command;

use regex::Regex;

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
/// `  Stream #0:0(und): Video: rawvideo (RGB[24] / 0x18424752), rgb24(pc, bt709, progressive), 1920x1080 [SAR 1:1 DAR 16:9], q=2-31, 99532 kb/s, 2 fps, 2 tbn (default)`
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
