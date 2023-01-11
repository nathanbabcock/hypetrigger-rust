// use crate::runner::{RunnerResult, RunnerResultV2};
// use std::{
//     io::Error,
//     sync::{
//         mpsc::{Receiver, TryRecvError},
//         Arc, Mutex,
//     },
//     thread::{self, JoinHandle},
// };

// pub type MessageQueue = Arc<Mutex<Vec<RunnerResult>>>;
// // pub type OnResult = fn(RunnerResult);
// pub type OnEmit = Arc<dyn Fn(RunnerResult) + Sync + Send>;

// pub type OnEmitV2<T> = Arc<dyn Fn(RunnerResultV2<T>) + Sync + Send>;

// pub enum MessageQueueCommand {
//     Exit,
// }

// pub fn emit_stdout(result: RunnerResult) {
//     println!("{:?}", result);
// }

// /// Buffers messages from the runner threads by periodically polling a Mutex
// /// - Receives: messages in the `message_queue` Mutex, or an Exit command
// /// - Sends: buffered collection of messages on periodic interval
// pub fn spawn_message_queue_thread(
//     rx: Receiver<MessageQueueCommand>,
//     message_queue: MessageQueue,
// ) -> Result<JoinHandle<()>, Error> {
//     thread::Builder::new()
//         .name("message_queue".into())
//         .spawn(move || message_queue_thread_inner(rx, message_queue))
// }

// /// Buffers messages from the runner threads by periodically polling a Mutex
// /// - Receives: messages in the `message_queue` Mutex, or an Exit command
// /// - Sends: buffered collection of messages on periodic interval
// pub fn message_queue_thread_inner(rx: Receiver<MessageQueueCommand>, message_queue: MessageQueue) {
//     loop {
//         let command = rx.try_recv();
//         match command {
//             Ok(MessageQueueCommand::Exit) => {
//                 println!("[message_queue] received stop command");
//                 break;
//             }
//             Err(TryRecvError::Disconnected) => {
//                 eprintln!("[message_queue] channel disconnected unexpectedly");
//                 break;
//             }
//             Err(TryRecvError::Empty) => {
//                 message_queue_handler(message_queue.clone());
//             }
//         }
//     }

//     println!("[message_queue] thread exiting");
// }

// pub fn message_queue_handler(message_queue: MessageQueue) {
//     let _queue = message_queue.lock().unwrap();

//     // todo something expensive
//     // e.g. serialize to base64 and send to Tauri frontend
//     // window.emit("message_queue", &queue);
// }
