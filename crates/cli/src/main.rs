use clap::Parser;
use std::path::Path;
use resize::resize_image_width;

const DESC: &str = "Content Aware Image Resizing using the seam carving algorithm.";

/// CAIRE resizing
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = Some(DESC))]
struct Args {
    /// Path of the image to resize
    #[clap(short, long, value_parser)]
    input: String,

    /// Output path
    #[clap(short, long, value_parser)]
    output: String,

    /// Resize width
    #[clap(short, long, value_parser)]
    width: u32

}

fn main() {
    let args = Args::parse();

    let input_image_path = Path::new(&args.input);
    let output_image_path = Path::new(&args.output);
    let resize_width = args.width;

    if !input_image_path.exists() {
        panic!("Input image does not exist!")
    }

    let img = image::open(input_image_path).unwrap().to_rgb8();
    let (img_width, _) = img.dimensions();

    if resize_width > img_width {
        panic!("Cannot upsample.")
    }

    let resized_image = resize_image_width(&img, resize_width);

    if let Ok(_) = resized_image.save(output_image_path) {
        eprintln!("Successfully resized image.");
    }

}

