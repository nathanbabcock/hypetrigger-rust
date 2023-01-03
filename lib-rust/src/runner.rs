use crate::{config::HypetriggerConfig, ffmpeg::RawImageData, trigger::Trigger};
use std::{
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc,
    },
    thread::{self, JoinHandle},
};

pub struct Runner {
    pub spawn: RunnerFn,
    pub thread: Option<WorkerThread>,
}

pub struct WorkerThread {
    pub tx: SyncSender<RunnerCommand>,
    pub join_handle: std::io::Result<JoinHandle<()>>,
}

/// Specifies all the context/state needed for a Runner to process a single frame
#[derive(Clone)]
pub struct RunnerContext {
    /// The config of the Job that is invoking this run
    pub config: Arc<HypetriggerConfig>,

    /// The specific Trigger that is currently being run
    pub trigger: Arc<dyn Trigger>,

    /// Raw pixels of the video frame
    pub image: RawImageData,

    /// Monotonically inreasing by 1, starting from 0. Does not correspond
    /// directly to the frame number of the source video, because it is
    /// (typically) sampled at a lower framerate.
    pub frame_num: u64,
}

impl RunnerContext {
    pub fn get_timestamp(&self) -> f64 {
        (self.frame_num as f64 / self.config.samplesPerSecond) as f64
    }
}

pub enum RunnerCommand {
    ProcessImage(RunnerContext),
    Exit,
    // NB: If it ever became necessary to add new *triggers* to an existing
    // runner, we could extend RunnerCommand:
    // - AddConfig(HypetriggerConfig),
    // - RemoveConfig(String),
}

#[derive(Debug)]
pub struct RunnerResult {
    pub text: String,
    pub frame_num: u64,
    pub trigger_id: String,
    pub input_id: String,
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct RunnerResultV2<T> {
    pub result: T,
    pub frame_num: u64,
    pub trigger_id: String,
    pub input_id: String,
    pub timestamp: f64,
}

pub type RunnerFn = fn(Receiver<RunnerCommand>, Arc<HypetriggerConfig>);
/// - Receives: either an image to process, or an exit command
/// - Sends: the recognized text
pub fn spawn_runner_thread(
    name: String,
    runner: RunnerFn,
    config: Arc<HypetriggerConfig>,
) -> WorkerThread {
    let (tx, rx) = sync_channel::<RunnerCommand>(0);
    let join_handle = thread::Builder::new()
        .name(name)
        .spawn(move || runner(rx, config));
    WorkerThread { tx, join_handle }
}
