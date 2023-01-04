use std::{
    collections::HashMap,
    error::Error,
    io::{self, Write},
    process::{Child, ChildStdin, Stdio},
    sync::{Arc, Mutex, RwLock},
    thread::JoinHandle,
};

use crate::{
    config::HypetriggerConfig,
    debugger::{Debugger, DebuggerRef},
    emit::OnEmit,
    ffmpeg::{
        on_ffmpeg_stderr, on_ffmpeg_stdout, send_ffmpeg_stop_signal, spawn_ffmpeg_childprocess,
        spawn_ffmpeg_stderr_thread, spawn_ffmpeg_stdout_thread, GetRunnerThread, OnFfmpegStderr,
        OnFfmpegStdout, SpawnFfmpegChildprocess, SpawnFfmpegStderrThread, SpawnFfmpegStdoutThread,
        StdioConfig,
    },
    runner::{spawn_runner_thread, RunnerFn, WorkerThread},
    tensorflow::TENSORFLOW_RUNNER,
    tesseract::TESSERACT_RUNNER,
};

pub type Jobs = HashMap<String, HypetriggerJob>;
pub type RunnerThreads = Arc<RwLock<HashMap<String, Arc<WorkerThread>>>>;
pub type OnPanic = Arc<dyn Fn(Box<dyn Error>) + Send + Sync>;

/// A multithreaded pipeline of execution
///
/// FFMPEG -> Tesseract/Tensorflow -> Emit
#[derive(Builder)]
pub struct Pipeline {
    // --- Config params ---
    /// Path to the FFMPEG executable (defaults to "ffmpeg" command in system PATH)
    #[builder(default = "\"ffmpeg\".into()")]
    ffmpeg_exe: String,

    /// Turn on or off different logging channels (ffmpeg, tesseract, tensorflow, etc.)
    #[builder(default = "Arc::new(RwLock::new(Debugger::default()))")]
    debugger: DebuggerRef,

    // --- Callbacks ---
    /// Callback that runs inside a Runner thread when a result for a frame has
    /// been obtained.
    ///
    /// - For Tesseract, this contains recognized text
    /// - For Tensorflow, this contains the image classification label & confidence
    /// - For custom Runners, it contains whatever data you pass along in the implementation
    ///
    /// Logs to console by default.
    /// @deprecated
    // #[builder(default = "Arc::new(emit_stdout)")]
    // on_emit: OnEmit,

    /// Callback for each line of FFMPEG stderr
    /// Useful for redirecting logs to program stdout or elsewhere,
    /// or extracting metadata or progress.
    ///
    /// If `None`, no thread is spawned to listen for FFMPEG stdout.
    #[builder(default = "Some(Arc::new(on_ffmpeg_stderr))")]
    on_ffmpeg_stderr: OnFfmpegStderr,

    /// Callback for each line of FFMPEG stdout.
    /// It includes the image pixels as `RawImageData`, and corresponding `Trigger`.
    ///
    /// The default implementation then forwards this to the appropriate Runner
    /// thread -- not typically changed.
    #[builder(default = "Arc::new(on_ffmpeg_stdout)")]
    on_ffmpeg_stdout: OnFfmpegStdout,

    // --- Other moduler core behavior ---
    #[builder(default = "Arc::new(spawn_ffmpeg_childprocess)")]
    spawn_ffmpeg_childprocess: SpawnFfmpegChildprocess,

    #[builder(default = "Arc::new(spawn_ffmpeg_stderr_thread)")]
    spawn_ffmpeg_stderr_thread: SpawnFfmpegStderrThread,

    #[builder(default = "Arc::new(spawn_ffmpeg_stdout_thread)")]
    spawn_ffmpeg_stdout_thread: SpawnFfmpegStdoutThread,

    #[builder(default = "Arc::new(spawn_runner_threads)")]
    spawn_runner_threads: SpawnRunnerThreads,

    /// Required in order to stop a job by sending commands to ffmpeg via stdin
    #[builder(default = "true")]
    enable_ffmpeg_stdin: bool,

    // --- Pipeline state ---
    /// Pointers to functions that spawn Runners,
    /// so that they can be called automatically when needed
    /// (eagerly at startup, or lazily when required by a job)
    #[builder(default = "HashMap::new()")]
    runners: HashMap<String, RunnerFn>,

    /// Tracks the current runner threads
    /// (e.g. Tensorflow, Tesseract, etc.)
    ///
    /// This must be kept separate from `Pipeline::runners` because the inner thread
    /// JoinHandles are not cloneable, so can't be used in the Builder.
    #[builder(setter(skip))]
    runner_threads: RunnerThreads,

    /// Tracks the currently running Jobs.
    /// Each job will have its own instance of FFMPEG,
    /// but will share runner threads.
    #[builder(setter(skip), default = "HashMap::new()")]
    jobs: Jobs,
}

impl PipelineBuilder {
    pub fn register_runner(&mut self, name: String, runner: RunnerFn) -> &mut Self {
        match self.runners {
            Some(ref mut hashmap) => hashmap.insert(name, runner),
            None => {
                self.runners = Some(HashMap::new());
                return self.register_runner(name, runner);
            }
        };
        self
    }
}

impl Pipeline {
    pub fn builder() -> PipelineBuilder {
        PipelineBuilder::default()
    }

    /// Starts a thread for the given Runner.
    /// If a thread for the given Runner already exists, nothing is changed.
    /// Runner must already be registered (e.g. a name mapped to a spawn function)
    pub fn spawn_runner(&mut self, name: String, config: Arc<HypetriggerConfig>) {
        if let Some(_) = self
            .runner_threads
            .read()
            .expect("acquire runner threads read lock")
            .get(&name)
        {
            return;
        }
        let runner_fn = *self
            .runners
            .get(&name)
            .unwrap_or_else(|| panic!("get runner fn for {}", name));
        let worker = spawn_runner_thread(name.clone(), runner_fn, config);
        self.runner_threads
            .write()
            .expect("acquire runner threads write lock")
            .insert(name, Arc::new(worker));
    }

    /// Spawns a thread for every registered Runner in the pipeline.
    /// These will idle and wait for input if no jobs are running yet.
    // pub fn spawn_all_runners(&mut self) {
    //     let keys = self.runners.keys().cloned().collect::<Vec<_>>();
    //     for name in keys {
    //         self.spawn_runner(name, config);
    //     }
    // }

    /// Spawns only the runners needed for a given job.
    pub fn spawn_runners_for_config(&mut self, config: Arc<HypetriggerConfig>) {
        for trigger in &config.triggers {
            self.spawn_runner(trigger.get_runner_type().clone(), config.clone());
        }
    }

    /// Determines which FFMPEG stdio channels to listen to,
    /// based on the Pipeline's registered callbacks.
    pub fn ffmpeg_stdio_config(&mut self) -> StdioConfig {
        StdioConfig {
            // stderr automatically disabled if unused
            stderr: if self.on_ffmpeg_stderr.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            },

            // stdin can be manually disabled (uncommon)
            stdin: if self.enable_ffmpeg_stdin {
                Stdio::piped()
            } else {
                Stdio::null()
            },

            // stdout is always on
            stdout: Stdio::piped(),
        }
    }

    /// Spawns an instance of FFMPEG, listens on stdio channels, and forwards decoded images to the appropriate Runner.
    ///
    /// The return result indicates the success of creating the Job, but the
    /// execution continues in separate threads.
    pub fn start_job(&mut self, job_id: String, config: HypetriggerConfig) -> io::Result<()> {
        let config_arc = Arc::new(config);

        // Validate job
        if self.jobs.contains_key(&job_id) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("job already exists with id {}", job_id),
            ));
        }
        if config_arc.triggers.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "job contains no triggers".to_string(),
            ));
        }

        // Get runners
        let runner_threads_clone = self.runner_threads.clone();
        let get_runner_thread: GetRunnerThread = Arc::new(move |name| -> Arc<WorkerThread> {
            runner_threads_clone
                .read()
                .expect("able to acquire lock on runner_threads (not poisoned)")
                .get(&name)
                .expect("hashmap contains a runner thread with the specified id")
                .clone()
        });

        // Ffmpeg childprocess
        let ffmpeg_stdio = self.ffmpeg_stdio_config();
        let mut ffmpeg_child = (self.spawn_ffmpeg_childprocess)(
            config_arc.clone(),
            ffmpeg_stdio,
            self.ffmpeg_exe.clone(),
            self.debugger.clone(),
        )?;
        let ffmpeg_stdin = ffmpeg_child.stdin.take();
        let ffmpeg_stderr = ffmpeg_child.stderr.take();
        let ffmpeg_stdout = match ffmpeg_child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "ffmpeg child should always have a piped stdout channel",
                ))
            }
        };

        // Handler for panic within threads
        let ffmpeg_child_arc = Arc::new(Mutex::new(ffmpeg_child));
        let ffmpeg_child_clone = ffmpeg_child_arc.clone();
        let on_panic: OnPanic = Arc::new(move |error| {
            eprintln!("[panic] Panic in thread: {}", error);
            eprintln!("killing ffmpeg childprocess...");
            let mut child = ffmpeg_child_clone.lock().unwrap();
            child.kill().unwrap();
            child.wait().unwrap();
            eprintln!("[panic] Ffmpeg childprocess killed");
        });

        // Ffmpeg stdout
        let ffmpeg_stdout_thread = match (self.spawn_ffmpeg_stdout_thread)(
            ffmpeg_stdout,
            config_arc.clone(),
            self.debugger.clone(),
            self.on_ffmpeg_stdout.clone(),
            get_runner_thread.clone(),
            on_panic.clone(),
        ) {
            Ok(thread) => thread,
            Err(err) => {
                // Clean up ffmpeg childprocess, which will now have nowhere to
                // put its stdout. Panic if this cleanup fails (it was already
                // plan B)
                let mut child = ffmpeg_child_arc.lock().unwrap();
                child
                    .kill()
                    .expect("ffmpeg process had started normally, so it should be killable");
                child
                    .wait()
                    .expect("no unknown error while waiting for ffmpeg process to exit");
                return Err(err);
            }
        };

        // Ffmpeg stderr
        // This one is tricky to unbox, since it's a Result inside an option
        // TODO: change this? It's un-ergonomic, and in the future we will
        // likely need to require a stderr handler in all cases (for things like
        // metadata and ffmpeg error detection).
        let ffmpeg_stderr_thread = match (self.spawn_ffmpeg_stderr_thread)(
            ffmpeg_stderr.unwrap(),
            config_arc.clone(),
            self.on_ffmpeg_stderr.clone(),
            self.debugger.clone(),
            on_panic.clone(),
        ) {
            Some(result) => match result {
                Ok(thread) => Some(thread),
                Err(err) => {
                    // Same thing as if stdout thread fails. In this case, the
                    // stdout thread will automatically terminate when the
                    // stdout channel closes.
                    let mut child = ffmpeg_child_arc.lock().unwrap();
                    child
                        .kill()
                        .expect("ffmpeg process had started normally, so it should be killable");
                    child
                        .wait()
                        .expect("no unknown error while waiting for ffmpeg process to exit");
                    ffmpeg_stdout_thread
                        .join()
                        .expect("ffmpeg stdout thread shouldn't panic, and should be joinable");
                    return Err(err);
                }
            },
            None => None,
        };

        // Spawn runner threads
        // TODO: error handling?
        self.spawn_runners_for_config(config_arc.clone());

        // Insert job
        let job = HypetriggerJob {
            config: config_arc,
            ffmpeg_child: ffmpeg_child_arc,
            ffmpeg_stdin: Mutex::new(ffmpeg_stdin),
            ffmpeg_stderr_thread,
            ffmpeg_stdout_thread,
        };
        self.jobs.insert(job_id, job);

        Ok(())
    }

    // TODO should return a result type
    pub fn stop_job(&mut self, job_id: String) {
        let job = self.jobs.remove(&job_id).expect("remove job from hashmap");
        let ffmpeg_stdin = job.ffmpeg_stdin.lock().unwrap();
        ffmpeg_stdin
            .as_ref()
            .expect("obtain ffmpeg stdin channel")
            .write_all(b"q\n")
            .expect("send quit signal");

        // join threads to block until job is definitely finished
        job.ffmpeg_stdout_thread
            .join()
            .expect("join ffmpeg stdout thread");
        if let Some(stderr_thread) = job.ffmpeg_stderr_thread {
            stderr_thread.join().expect("join ffmpeg stderr thread");
        }
    }

    pub fn stop_all_jobs(&mut self) -> Result<(), String> {
        let keys = self.jobs.keys().cloned().collect::<Vec<_>>();
        for key in keys {
            self.stop_job(key);
        }
        Ok(())
    }
}

pub fn test_builder() {
    let _builder = Pipeline::builder().build().unwrap();
}

struct HypetriggerJob {
    pub ffmpeg_child: Arc<Mutex<Child>>,
    pub ffmpeg_stdin: Mutex<Option<ChildStdin>>,
    pub ffmpeg_stderr_thread: Option<JoinHandle<()>>,
    pub ffmpeg_stdout_thread: JoinHandle<()>,
    pub config: Arc<HypetriggerConfig>,
}

pub type SpawnRunnerThreads = Arc<
    dyn (Fn(
            &HypetriggerConfig,
            &HashMap<String, WorkerThread>,
            OnEmit,
        ) -> HashMap<String, WorkerThread>)
        + Send
        + Sync,
>;

/// Spawns the default runners: Tensorflow and Tesseract
/// Source is in pipeline.rs rather than runner.rs to hopefully avoid
/// unnecessary imports of Tesseract and Tensorflow
fn spawn_runner_threads(
    config: &HypetriggerConfig,
    _runners: &HashMap<String, WorkerThread>,
    _on_result: OnEmit,
) -> HashMap<String, WorkerThread> {
    let hashmap: HashMap<String, WorkerThread> = HashMap::new();
    // hashmap.extend(runners.into_iter());
    // TODO !!! IMPORTANT runners.clone()
    for trigger in &config.triggers {
        if hashmap.contains_key(trigger.get_runner_type().as_str()) {
            continue;
        }
        match trigger.get_runner_type().as_str() {
            TENSORFLOW_RUNNER => {
                //   hashmap.insert(
                //     "tensorflow".into(),
                //     spawn_tensorflow_thread(config.clone(), self.on_result),
                // )
            }
            TESSERACT_RUNNER => {
                // hashmap.insert("tesseract".into(), spawn_tesseract_thread(self.on_result))
            }
            unknown => panic!("Unknown runner type: {}", unknown),
        };
    }
    hashmap
}
