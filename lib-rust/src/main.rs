use std::cell::RefCell;

use hypetrigger::{
    pipeline_simple::{Hypetrigger, TesseractTrigger},
    tesseract::init_tesseract,
};
use tesseract::Tesseract;

fn main() {
    println!("Hello world!");

    let tesseract = RefCell::new(Some(Tesseract::new(None, None).unwrap()));
    let trigger = TesseractTrigger {
        tesseract,
        crop: None,
        threshold_filter: None,
        callback: None,
    };

    match Hypetrigger::new()
        .set_input("D:/My Videos Backup/OBS/Road to the 20-Bomb/17.mp4".to_string())
        .add_trigger(Box::new(trigger))
        .run()
    {
        Ok(_) => println!("[main] done"),
        Err(e) => eprintln!("[main] error: {:?}", e),
    }
}
