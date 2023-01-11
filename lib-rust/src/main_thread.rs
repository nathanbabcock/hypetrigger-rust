// // TODO this is unnecessary or extra?
// // Or should be included as an optional utility to facilitate useage w/ Tauri?

// use std::{io::Error, sync::mpsc::Receiver, thread::JoinHandle};

// pub enum MainThreadCommand {
//     Start,
//     Stop(String),
//     Exit,
// }

// /// - Receives: start, stop, and exit commands
// /// - Sends: Spawns ffmpeg and connects them to all threads
// pub fn spawn_main_thread(_rx: Receiver<MainThreadCommand>) -> Result<JoinHandle<()>, Error> {
//     todo!()
// }

// /// - Receives: start, stop, and exit commands
// /// - Sends: Spawns ffmpeg and connects them to all threads
// pub fn main_thread_inner(_rx: Receiver<MainThreadCommand>) {
//     todo!()
// }
