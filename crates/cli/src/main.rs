// use clap::Parser;
// use std::path::Path;
// use resize::resize_image_width;
use eframe::egui;
use egui_extras::{ RetainedImage, image as eeimage };

const DESC: &str = "Content Aware Image Resizing using the seam carving algorithm.";

// /// CAIRE resizing
// #[derive(Parser, Debug)]
// #[clap(author, version, about, long_about = Some(DESC))]
// struct Args {
//     /// Path of the image to resize
//     #[clap(short, long, value_parser)]
//     input: String,

//     /// Output path
//     #[clap(short, long, value_parser)]
//     output: String,

//     /// Resize width
//     #[clap(short, long, value_parser)]
//     width: u32

// }

// fn main() {
//     let args = Args::parse();

//     let input_image_path = Path::new(&args.input);
//     let output_image_path = Path::new(&args.output);
//     let resize_width = args.width;

//     if !input_image_path.exists() {
//         panic!("Input image does not exist!")
//     }

//     let img = image::open(input_image_path).unwrap().to_rgb8();
//     let (img_width, _) = img.dimensions();

//     if resize_width > img_width {
//         panic!("Cannot upsample.")
//     }

//     let resized_image = resize_image_width(&img, resize_width);

//     if let Ok(_) = resized_image.save(output_image_path) {
//         eprintln!("Successfully resized image.");
//     }

// }



fn main() {
    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        ..Default::default()
    };
    eframe::run_native(
        "Native file dialogs and drag-and-drop files",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

#[derive(Default)]
struct MyApp {
    dropped_files: Vec<egui::DroppedFile>,
    picked_path: Option<String>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Drag-and-drop files onto the window!");

            if ui.button("Open file…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.picked_path = Some(path.display().to_string());
                }
            }

            if let Some(picked_path) = &self.picked_path {

                let image = image::open(picked_path).unwrap().to_rgba8();
                let size = [image.dimensions().0 as _, image.dimensions().1 as _];
                let pixels = image.as_flat_samples();

                let display_image = egui::ColorImage::from_rgba_unmultiplied(
                    size,
                    pixels.as_slice(),
                );
                
                let mut texture = None;
                let texture: &egui::TextureHandle = texture.get_or_insert_with(|| {
                    // Load the texture only once.
                    // ui.ctx().load_texture("my-image", egui::ColorImage::example())
                    ui.ctx().load_texture("my-image", display_image)
                });

                // Show the image:
                ui.image(texture, texture.size_vec2());
            }
        

            // Show dropped files (if any):
            if !self.dropped_files.is_empty() {
                ui.group(|ui| {
                    ui.label("Dropped files:");

                    for file in &self.dropped_files {
                        let mut info = if let Some(path) = &file.path {
                            path.display().to_string()
                        } else if !file.name.is_empty() {
                            file.name.clone()
                        } else {
                            "???".to_owned()
                        };
                        if let Some(bytes) = &file.bytes {
                            use std::fmt::Write as _;
                            write!(info, " ({} bytes)", bytes.len()).ok();
                        }
                        ui.label(info);

                    }
                });
            }

        });

        preview_files_being_dropped(ctx);

        // Collect dropped files:
        if !ctx.input().raw.dropped_files.is_empty() {
            self.dropped_files = ctx.input().raw.dropped_files.clone();
        }
    }
}

/// Preview hovering files:
fn preview_files_being_dropped(ctx: &egui::Context) {
    use egui::*;
    use std::fmt::Write as _;

    if !ctx.input().raw.hovered_files.is_empty() {
        let mut text = "Dropping files:\n".to_owned();
        for file in &ctx.input().raw.hovered_files {
            if let Some(path) = &file.path {
                write!(text, "\n{}", path.display()).ok();
            } else if !file.mime.is_empty() {
                write!(text, "\n{}", file.mime).ok();
            } else {
                text += "\n???";
            }
        }

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.input().screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}
