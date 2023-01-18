use crate::error::{Error, Result};
use crate::trigger::{Frame, Trigger};
use std::{
    sync::{mpsc::SyncSender, Arc},
    thread::{self, JoinHandle},
};

/// A wrapper around any other Trigger that sends it across a channel to run on
/// a separate thread.
#[derive(Clone)]
pub struct AsyncTrigger {
    pub trigger: Arc<dyn Trigger>,
    pub runner_thread: Arc<TriggerThread>,
}

impl Trigger for AsyncTrigger {
    fn on_frame(&self, frame: &Frame) -> Result<()> {
        self.runner_thread
            .tx
            .send(TriggerCommand::Packet(TriggerPacket {
                frame: frame.clone(),
                trigger: self.trigger.clone(),
            }))
            .map_err(Error::from_std)
    }
}

impl AsyncTrigger {
    pub fn from_trigger<T>(trigger: T, runner_thread: Arc<TriggerThread>) -> Self
    where
        T: Trigger + 'static,
    {
        Self {
            trigger: Arc::new(trigger),
            runner_thread,
        }
    }
}

/// A separate thread that runs one or more `AsyncTriggers`, by receiving them
/// over a channel, paired with the frame to process.
pub struct TriggerThread {
    pub tx: SyncSender<TriggerCommand>,
    pub join_handle: JoinHandle<()>,
}

impl TriggerThread {
    /// Prepares a new thread capable of running Triggers, including the
    /// communication channels, spawning the thread itself, and wrapping the
    /// whole struct in an `Arc`.
    pub fn spawn() -> Arc<Self> {
        let (tx, rx) = std::sync::mpsc::sync_channel::<TriggerCommand>(100);
        let join_handle = thread::spawn(move || {
            println!("[trigger_thread] Listening for async trigger commands.");
            while let Ok(command) = rx.recv() {
                match command {
                    TriggerCommand::Stop => {
                        println!("[trigger_thread] Received stop command.");
                        break;
                    }
                    TriggerCommand::Packet(payload) => {
                        let result = payload.trigger.on_frame(&payload.frame);
                        if let Err(e) = result {
                            eprintln!("Error in async trigger: {}", e);
                        }
                    }
                }
            }
            println!("[trigger_thread] Exiting.");
        });
        Arc::new(Self { tx, join_handle })
    }
}

/// A command send over a channel to a `TriggerThread`
pub enum TriggerCommand {
    /// Tell the thread to clean up and exit
    Stop,

    /// Tell the thread to run a trigger
    Packet(TriggerPacket),
}

/// Everything a `TriggerThread` needs to run a `AsyncTrigger`
#[derive(Clone)]
pub struct TriggerPacket {
    frame: Frame,
    trigger: Arc<dyn Trigger>,
}
