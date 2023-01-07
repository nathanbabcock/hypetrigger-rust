use crate::config::HypetriggerConfig;
use std::os::windows::process::CommandExt;
use std::process::Stdio;
use std::{
    io,
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
};

//// Job
pub struct HypetriggerJob {
    pub config: HypetriggerConfig,
    pub ffmpeg_child: Child,
    pub ffmpeg_stdin: Option<ChildStdin>,
    pub ffmpeg_stdout: Option<ChildStdout>,
    pub ffmpeg_stderr: Option<ChildStderr>,
}

//// Pipeline

pub struct PipelineSimple;

impl PipelineSimple {
    /// Path to the FFMPEG executable (defaults to "ffmpeg" command in system PATH)
    pub fn get_ffmpeg_exe(&self) -> String {
        "ffmpeg".to_string()
    }

    pub fn spawn_ffmpeg_childprocess(&self, config: &HypetriggerConfig) -> io::Result<Child> {
        // config parameters
        let input_video = config.inputPath.as_str();
        let samples_per_second = config.samplesPerSecond;
        let num_triggers = config.triggers.len();

        // construct filter graph
        let mut filter_complex: String =
            format!("[0:v]fps={},split={}", samples_per_second, num_triggers);
        for i in 0..num_triggers {
            filter_complex.push_str(format!("[in{}]", i).as_str());
        }
        filter_complex.push(';');
        for i in 0..num_triggers {
            let trigger = &config.triggers[i];
            let in_w = trigger.get_crop().width;
            let in_h = trigger.get_crop().height;
            let x = trigger.get_crop().x;
            let y = trigger.get_crop().y;

            filter_complex.push_str(
                format!(
                    "[in{}]crop={}:{}:{}:{}:exact=1[out{}]",
                    i, in_w, in_h, x, y, i
                )
                .as_str(),
            );
            if i < num_triggers - 1 {
                filter_complex.push(';');
            }
        }

        // retrieve ffmpeg path
        let ffmpeg_path = self.get_ffmpeg_exe();
        println!("[ffmpeg] using exe: {}", ffmpeg_path);

        // spawn command
        let mut cmd = Command::new(ffmpeg_path);
        cmd.arg("-hwaccel")
            .arg("auto")
            .arg("-i")
            .arg(input_video)
            .arg("-filter_complex")
            .arg(filter_complex.clone());

        for i in 0..num_triggers {
            cmd.arg("-map").arg(format!("[out{}]", i));
        }

        // debug output
        // TODO rewrite to a self.debug_ffmpeg_command() function
        // (or utility function) that iterates over the args and prints them
        println!("[ffmpeg] debug command:");
        println!("ffmpeg \\");
        println!("  -hwaccel auto \\");
        println!("  -i \"{}\" \\", input_video);
        println!("  -filter_complex \"{}\" \\", filter_complex);
        for i in 0..num_triggers {
            println!("  -map [out{}] \\", i);
        }
        println!("  -vsync drop \\");
        println!("  -vframes {} \\", num_triggers * 5);
        println!("  -an -y \\");
        println!("  \"scripts/frame%03d.bmp\"");

        // add arguments
        cmd.arg("-vsync")
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
            .creation_flags(0x08000000)
            .spawn()
    }

    pub fn start_job(&self, config: HypetriggerConfig) -> Result<HypetriggerJob, io::Error> {
        let mut ffmpeg_child = self.spawn_ffmpeg_childprocess(&config)?;
        let ffmpeg_stdin = ffmpeg_child.stdin.take();
        let ffmpeg_stderr = ffmpeg_child.stderr.take();
        let ffmpeg_stdout = ffmpeg_child.stdout.take();

        let job = HypetriggerJob {
            config,
            ffmpeg_child,
            ffmpeg_stdin,
            ffmpeg_stderr,
            ffmpeg_stdout,
        };
        Ok(job)
    }
}
