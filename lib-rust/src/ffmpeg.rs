use crate::config::HypetriggerConfig;
use crate::logging::LoggingConfig;
use crate::runner::{RunnerCommand, RunnerContext, WorkerThread};
use crate::trigger::Trigger;

use std::io::{BufRead, BufReader, Error, Read, Write};
use std::os::windows::process::CommandExt;

use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{mpsc::Receiver, Arc};
use std::thread;
use std::thread::JoinHandle;

pub type RawImageData = Arc<Vec<u8>>;

pub enum FfmpegStdinCommand {
    Stop,
}

/// Specifies whether to attach to each stdio channel or not
pub struct StdioConfig {
    pub stdin: Stdio,
    pub stdout: Stdio,
    pub stderr: Stdio,
}

pub type SpawnFfmpegChildprocess = Arc<
    dyn (Fn(Arc<HypetriggerConfig>, StdioConfig, String) -> Result<Child, Error>) + Sync + Send,
>;
/// Generates and runs an FFMPEG command similar to this one (in the case of two inputs):
///
/// ```
/// ffmpeg \
///  -hwaccel cuda \
///  -i "F:/OBS/Road to the 20-Bomb/17.mp4" \
///  -filter_complex "[0:v]fps=2,split=2[in1][in2];[in1]crop=60.59988:60.59988:930.70038:885.6,scale=224:224,negate[out1];[in2]crop=2:2:0:0,scale=224:224[out2];[out1][out2]" \
///  -map "[out0]" \
///  -map "[out1]" \
///  -f rawvideo \
///  -pix_fmt rgb24 \
///  -an -y pipe:1 > "scripts/raw.bin"
/// ```
///
/// Explanation of all FFMPEG parameters:
/// - `-hwaccel cuda` (or `-hwaccel auto`) run on the GPU
/// - `-i path/to/file.mp4` reads the input video
/// - `-filter_complex` transforms every frame into the format expected by tesseract or tensorflow
///   - `fps=x` drops the fps to the sample rate, skipping all other frames
///   - `split=n` splits video for every trigger
///   - `crop` isolates the rectangle identified in trigger config `cropFunction`
///   - `scale` only applies to tensorflow, and resizes output to 224x224 expected by the NN
/// - `-map [outN]` creates one output stream for each branch in the filter graph
/// - `-vsync drop` *important* drops frame timestamps, needed for `interleave` filter to behave as expected
/// - `-f rawvideo` since no output file is specified, tell FFMPEG which video format to use (raw bytes)
/// - `-pix_fmt rgb24` 1 byte per pixel, 3 channels, RGB
/// - `-an` drop audio
/// - `-y` *unneccessary* overwrite output file if it exists (irrelevant in this case)
/// - `-pipe:1` output to stdout (this will be consumed on another thread for processing)
///
pub fn spawn_ffmpeg_childprocess(
    config: Arc<HypetriggerConfig>,
    stdio_config: StdioConfig,
    ffmpeg_exe: String,
) -> Result<Child, Error> {
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
    let ffmpeg_path_str = ffmpeg_exe.as_str();
    if config.logging.debug_ffmpeg {
        println!("[ffmpeg] exe: {}", ffmpeg_path_str);
    }

    // spawn command
    let mut cmd = Command::new(ffmpeg_path_str);
    cmd.arg("-hwaccel")
        .arg("auto")
        .arg("-i")
        .arg(input_video)
        .arg("-filter_complex")
        .arg(filter_complex.clone());

    for i in 0..num_triggers {
        cmd.arg("-map").arg(format!("[out{}]", i));
    }

    // add arguments
    let child = cmd
        .arg("-vsync")
        .arg("drop")
        .arg("-f")
        .arg("rawvideo")
        .arg("-pix_fmt")
        .arg("rgb24")
        .arg("-an")
        .arg("-y")
        .arg("pipe:1")
        .stdin(stdio_config.stdin)
        .stdout(stdio_config.stdout)
        .stderr(stdio_config.stderr)
        .creation_flags(0x08000000)
        .spawn();

    // debug output
    if config.logging.debug_ffmpeg {
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
    }

    child
}

/// Function signature for spawning a thread to process ffmpeg stderr
pub type SpawnFfmpegStderrThread = Arc<
    dyn (Fn(
            ChildStderr,
            Arc<HypetriggerConfig>,
            OnFfmpegStderr,
        ) -> Option<Result<JoinHandle<()>, Error>>)
        + Sync
        + Send,
>;

/// Optional thread to process stderr from ffmpeg. It will automatically terminate
/// when the ffmpeg process exits.
///
/// FFMPEG sends all logs to stderr (not necessarily just errors)
/// We pipe these and read them async to extract info like video duration,
/// or re-routing ffmpeg logs to println.
///
/// If no callback is provided, the thread won't be spawned.
///
/// - Recieves: lines from ffmpeg stderr
/// - Sends: Nothing/calls callback on each line
pub fn spawn_ffmpeg_stderr_thread(
    stderr: ChildStderr,
    config: Arc<HypetriggerConfig>,
    on_ffmpeg_stderr: OnFfmpegStderr,
) -> Option<Result<JoinHandle<()>, Error>> {
    let logging = config.logging.clone();
    on_ffmpeg_stderr.map(|on_ffmpeg_stderr| {
        thread::Builder::new()
            .name("ffmpeg_stderr".into())
            .spawn(move || {
                BufReader::new(stderr)
                    .lines()
                    .for_each(|line| (on_ffmpeg_stderr)(line, config.clone()));
                if logging.debug_thread_exit {
                    println!("[ffmpeg.stderr] done; thread exiting");
                }
            })
    })
}

/// Callback for each line of FFMPEG stderr
pub type OnFfmpegStderr =
    Option<Arc<dyn Fn(Result<String, Error>, Arc<HypetriggerConfig>) + Send + Sync>>;

/// Callback for every line of ffmpeg stderr
pub fn on_ffmpeg_stderr(line: Result<String, Error>, _config: Arc<HypetriggerConfig>) {
    match line {
        Ok(string) => println!("{}", string),
        Err(error) => eprintln!("{}", error),
    }
}

pub type SpawnFfmpegStdoutThread = Arc<
    dyn (Fn(
            ChildStdout,
            Arc<HypetriggerConfig>,
            OnFfmpegStdout,
            GetRunnerThread,
        ) -> Result<JoinHandle<()>, Error>)
        + Sync
        + Send,
>;

/// Handles receiving raw pixel data from FFMPEG on the stdout channel
/// and mapping it to the corresponding trigger config.
pub fn spawn_ffmpeg_stdout_thread(
    mut stdout: ChildStdout,
    config: Arc<HypetriggerConfig>,
    on_ffmpeg_stdout: OnFfmpegStdout,
    get_runner: GetRunnerThread,
) -> Result<JoinHandle<()>, Error> {
    thread::Builder::new()
        .name("ffmpeg_stdout".into())
        .spawn(move || {
            // Init buffers
            let mut buffers: Vec<Vec<u8>> = Vec::new();
            for trigger in &config.triggers {
                let width = trigger.get_crop().width;
                let height = trigger.get_crop().height;
                const CHANNELS: u32 = 3;
                let buf_size = (width * height * CHANNELS) as usize;
                if config.logging.debug_buffer_allocation {
                    println!(
                        "[rust] Allocated buffer of size {} for trigger id ",
                        buf_size // trigger.id, // todo no more trigger.id
                    );
                }
                buffers.push(vec![0_u8; buf_size]);
            }

            // Listen for data
            let mut frame_num = 0;
            let mut trigger_index = 0;
            let num_triggers = config.triggers.len();
            while stdout.read_exact(&mut buffers[trigger_index]).is_ok() {
                let cur_trigger = &config.triggers[trigger_index];
                let clone = buffers[trigger_index].clone(); // Necessary?
                let raw_image_data: RawImageData = Arc::new(clone);

                let context = RunnerContext {
                    config: config.clone(),
                    image: raw_image_data,
                    trigger: cur_trigger.clone(),
                    frame_num: frame_num as u64,
                };

                on_ffmpeg_stdout(context, get_runner.clone());
                trigger_index += 1;
                if trigger_index >= num_triggers {
                    trigger_index = 0;
                    frame_num += 1;
                }
            }

            if config.logging.debug_thread_exit {
                println!("[ffmpeg] done; thread exiting");
            }
        })
}

pub type GetRunnerThread = Arc<dyn (Fn(String) -> Arc<WorkerThread>) + Sync + Send>;
pub type OnFfmpegStdout = Arc<dyn Fn(RunnerContext, GetRunnerThread) + Sync + Send>;
pub fn on_ffmpeg_stdout(context: RunnerContext, get_runner: GetRunnerThread) {
    // TODO num_triggers went out of scope
    // if config.logging.debug_buffer_transfer {
    //     println!(
    //         "[ffmpeg] read {} bytes for trigger {}",
    //         buffers[cur_frame % num_triggers].len(),
    //         cur_trigger.id
    //     );
    // }

    let tx_name = &context.trigger.get_runner_type();
    let tx = get_runner(tx_name.clone()).tx.clone();

    if context.config.logging.debug_buffer_transfer {
        println!(
            "[ffmpeg] sending {} bytes to {} for trigger ",
            context.image.len(),
            tx_name, //cur_trigger.id, // TODO no more id
        );
    }

    tx.send(RunnerCommand::ProcessImage(context))
        .expect("send image buffer");
}

pub fn spawn_ffmpeg_stdin_thread(
    mut stdin: ChildStdin,
    rx: Receiver<FfmpegStdinCommand>,
) -> Result<JoinHandle<()>, Error> {
    thread::Builder::new()
        .name("ffmpeg_stdin".into())
        .spawn(move || {
            while let Ok(command) = rx.recv() {
                match command {
                    FfmpegStdinCommand::Stop => {
                        stdin.write_all(b"q").expect("write to ffmpeg stdin");
                    }
                }
            }
            // while let Ok(Stop) = rx.recv() {
            //     println!("[ffmpeg.stdin] Sending quit signal");
            //     stdin.write_all(b"q\n").expect("send quit signal");
            // }
        })
}
