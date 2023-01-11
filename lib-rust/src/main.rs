use std::cell::RefCell;

use hypetrigger::{
    error::{Error, Result},
    photon::Crop,
    pipeline_simple::{Hypetrigger, RunnerThread, ThreadTrigger},
    simple_trigger::SimpleTrigger,
    tesseract::{init_tesseract, TesseractTrigger},
};
use tesseract::Tesseract;

fn main() -> Result<()> {
    Hypetrigger::new()
        .set_input("D:/My Videos Backup/OBS/Road to the 20-Bomb/17.mp4".to_string())
        .add_trigger(SimpleTrigger::new(|frame| {
            println!(
                "received frame {}: {}x{}",
                frame.frame_num,
                frame.image.width(),
                frame.image.height()
            );
            // Now do whatever you want with it...
        }))
        .run()
        .map_err(Error::from_display)
}

fn main_tesseract() -> Result<()> {
    println!("Hello world!");

    let tesseract = init_tesseract(None, None)?;
    let trigger = TesseractTrigger {
        tesseract,
        crop: Some(Crop {
            left_percent: 25.0,
            top_percent: 25.0,
            width_percent: 10.0,
            height_percent: 10.0,
        }),
        threshold_filter: None,
        callback: None,
    };

    Hypetrigger::new()
        .set_input("D:/My Videos Backup/OBS/Road to the 20-Bomb/17.mp4".to_string())
        .add_trigger(trigger)
        .run()
        .map_err(Error::from_display)
}

fn main_threaded() -> Result<()> {
    println!("Hello world!");

    let runner_thread = RunnerThread::spawn();
    let tesseract = init_tesseract(None, None)?;
    let base_trigger = TesseractTrigger {
        tesseract,
        crop: Some(Crop {
            left_percent: 25.0,
            top_percent: 25.0,
            width_percent: 10.0,
            height_percent: 10.0,
        }),
        threshold_filter: None,
        callback: None,
    };
    let trigger = ThreadTrigger::new(base_trigger, runner_thread.clone());

    Hypetrigger::new()
        .set_input("D:/My Videos Backup/OBS/Road to the 20-Bomb/17.mp4".to_string())
        .add_trigger(trigger)
        .run()
        .map_err(Error::from_display)
}
