use crate::{config::HypetriggerConfig, emit::OnEmit, ffmpeg::RawImageData, trigger::Trigger};
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

pub struct ProcessImagePayload {
    pub input_id: String,
    pub image: RawImageData,
    pub trigger: Arc<Trigger>,
    // pub trigger_id: String,
    // pub width: u32,
    // pub height: u32,

    // // pass entire dyn Trigger instance instead?
    // pub filter: Option<ThresholdFilter>, // needs to go...
}
pub enum RunnerCommand {
    ProcessImage(ProcessImagePayload),
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

pub type RunnerFn = fn(Receiver<RunnerCommand>, OnEmit, Arc<HypetriggerConfig>);
/// - Receives: either an image to process, or an exit command
/// - Sends: the recognized text
pub fn spawn_runner_thread(
    name: String,
    on_result: OnEmit,
    runner: RunnerFn,
    config: Arc<HypetriggerConfig>,
) -> WorkerThread {
    let (tx, rx) = sync_channel::<RunnerCommand>(0);
    let join_handle = thread::Builder::new()
        .name(name)
        .spawn(move || runner(rx, on_result, config.clone()));
    WorkerThread { tx, join_handle }
}
