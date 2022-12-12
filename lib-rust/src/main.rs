extern crate photon_rs;
use photon_rs::native::{open_image, save_image};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");

    // Open the image (a PhotonImage is returned)
    let mut img = open_image("F:\\Screenshots\\hypetrigger-apex-stat-trackers.png")?;
    println!("{}x{}", img.get_width(), img.get_height());

    // let image_data = img.get_image_data();

    // Increment the red channel by 40
    photon_rs::channels::alter_red_channel(&mut img, 40);

    // Write file to filesystem.
    save_image(img, "output/raw_image.png");

    Ok(())
}
