use hypetrigger::ffmpeg::command::FfmpegCommand;
use hypetrigger::ffmpeg::error::Result;
use hypetrigger::ffmpeg::event::OutputVideoFrame;
use hypetrigger::image::HypetriggerImage;

fn main() -> Result<()> {
  println!("Hello, world!");

  let trigger = |frame: OutputVideoFrame| HypetriggerImage::from(frame);

  let iter = FfmpegCommand::new()
    .testsrc()
    .rawvideo()
    .spawn()?
    .iter()?
    .filter_frames()
    // .map(|frame| trigger.run(frame))
    .map(trigger)
    .for_each(|e| println!("frame: {}x{}", e.get_width(), e.get_height()));

  Ok(())
}
