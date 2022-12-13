/// Specify which events are logged to stdout
/// Having all messages enabled with high throughput is impossible to read,
/// and can even cause slowdown when rendering log output in VSCode terminal or elsewhere.
///
/// Enable only the messages you need to debug your code; a few informative
/// defaults are enabled by default.
#[derive(Copy, Clone, Debug)]
pub struct LoggingConfig {
    /// print ffmpeg binary path and test command
    pub debug_ffmpeg: bool,

    /// print size of each allocated buffer for image data coming from ffmpeg
    pub debug_buffer_allocation: bool,

    /// print every buffer sent or received across channels between threads
    pub debug_buffer_transfer: bool,

    //// print message when each thread terminates
    pub debug_thread_exit: bool,

    /// redirect ffmpeg metadata and progress logs to stdout
    pub log_ffmpeg_stderr: bool,
}

impl LoggingConfig {
    // TODO implement Default trait
    pub fn default() -> LoggingConfig {
        LoggingConfig {
            // on
            debug_thread_exit: true,

            // off
            debug_ffmpeg: false,
            debug_buffer_allocation: false,
            debug_buffer_transfer: false,
            log_ffmpeg_stderr: false,
        }
    }
}
