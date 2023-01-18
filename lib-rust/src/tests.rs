use crate::{
    async_trigger::{AsyncTrigger, TriggerThread},
    error::{Error, Result},
    photon::Crop,
    pipeline::Hypetrigger,
    simple_trigger::SimpleTrigger,
    tesseract::{init_tesseract, TesseractTrigger},
};

#[test]
fn simple_trigger() -> Result<()> {
    Hypetrigger::new()
        .test_input()
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

#[test]
fn tesseract() -> Result<()> {
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
        enable_debug_breakpoints: false,
    };

    Hypetrigger::new()
        .test_input()
        .add_trigger(trigger)
        .run()
        .map_err(Error::from_display)
}

#[test]
fn async_trigger() -> Result<()> {
    let runner_thread = TriggerThread::spawn();
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
        enable_debug_breakpoints: false,
    };
    let trigger = AsyncTrigger::from_trigger(base_trigger, runner_thread);

    Hypetrigger::new()
        .test_input()
        .add_trigger(trigger)
        .run()
        .map_err(Error::from_display)
}
