use eframe::egui::{self, ColorImage, Color32, Vec2};
use image::RgbImage;
use std::sync::mpsc::{channel, Receiver, Sender};
use resize::{
    calculate_energy_map,
    find_low_energy_seam,
    delete_seam
};

fn main() {
    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some(Vec2::new(1400., 700.)),
        ..Default::default()
    };
    eframe::run_native(
        "CAIRE",
        options,
        Box::new(|_cc| Box::new(App::default())),
    );
}

struct App {
    dropped_files: Vec<egui::DroppedFile>,
    picked_path: Option<String>,
    selected_image: Option<RgbImage>,
    selected_image_texture: Option<egui::TextureHandle>,
    resized_image_texture: Option<egui::TextureHandle>,
    resize_width: u32,
    send_resize: Sender<RgbImage>,
    receive_resize: Receiver<RgbImage>
}

impl Default for App {
    fn default() -> Self {
        let (send, recv) = channel();
        Self {
            dropped_files: vec![],
            picked_path: None,
            selected_image: None,
            selected_image_texture: None,
            resized_image_texture: None,
            resize_width: 0,
            send_resize: send,
            receive_resize: recv
        }
    }

}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        egui::CentralPanel::default().show(ctx, |ui| {
            // ui.label("Drag-and-drop files onto the window!");

            if ui.button("Open file…").clicked() {

                self.selected_image = None;
                self.selected_image_texture = None;
                self.resized_image_texture = None;

                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.picked_path = Some(path.display().to_string());

                    let image = image::open(path).unwrap().to_rgb8();
                    self.selected_image = Some(image.clone());
                    let size = [image.dimensions().0 as _, image.dimensions().1 as _];
                    let pixels = image.as_flat_samples();

                    let display_image = egui::ColorImage::from_rgb(
                        size,
                        pixels.as_slice(),
                    );
                    
                    let selected_image_texture = ui.ctx().load_texture("my-image", display_image);

                    self.selected_image_texture = Some(selected_image_texture);
                }
            }

            if let Some(texture) = &self.selected_image_texture {
                ui.add(egui::Slider::new(&mut self.resize_width, 0..=texture.size()[0] as u32).text("My value"));
                if ui.add(egui::Button::new("Resize")).clicked() {
                    let sender = self.send_resize.clone();

                    let image = self.selected_image.clone().unwrap();
                    let resize_width = self.resize_width;
                    rayon::spawn(move || {
                        resize_image(
                            &image,
                            resize_width,
                            sender
                        )
                    });
                }
            }

            if let Some(image) = self.receive_resize.try_iter().last() {
                let size = [image.dimensions().0 as _, image.dimensions().1 as _];
                let pixels = image.as_flat_samples();
                let display_image = egui::ColorImage::from_rgb(
                    size,
                    pixels.as_slice(),
                );
                let resized_image_texture: egui::TextureHandle = 
                    ui.ctx().load_texture("my-image", display_image).clone();
                
                self.resized_image_texture = Some(resized_image_texture);
            }

            if let Some(texture) = self.resized_image_texture.clone() {
                // Show the image:
                let w_height = ui.available_height();
                let w_width = ui.available_width();
                let Vec2 { x: width, y: height } = texture.size_vec2();

                let (diplay_width, display_height) = 
                    resize_image_for_ui((width, height), (w_width, w_height));
                ui.image(&texture, (diplay_width, display_height));
            } else if let Some(texture) = self.selected_image_texture.clone() {
                // Show the image:
                let w_height = ui.available_height();
                let w_width = ui.available_width();
                let Vec2 { x: width, y: height } = texture.size_vec2();

                let (diplay_width, display_height) = 
                    resize_image_for_ui((width, height), (w_width, w_height));
                
                ui.image(&texture, (diplay_width, display_height));
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

fn resize_image(
    img: &RgbImage,
    to_width: u32,
    sender: Sender<RgbImage>
) -> () {
    let img_size = img.dimensions();
    let mut new_size = (img_size.0, img_size.1);
    let mut img = img.clone();
    for _ in 0..img_size.0 - to_width {
        let energy_map = calculate_energy_map(&img, new_size);
        let seam = find_low_energy_seam(&energy_map, new_size);
        img = delete_seam(&img, &seam);
        new_size.0 -= 1;
        sender.send(img.clone()).ok().unwrap()
    }
}

trait ColorImageFrom {
    fn from_rgb(size: [usize; 2], rgb: &[u8]) -> Self;
}

impl ColorImageFrom for ColorImage {
    fn from_rgb(size: [usize; 2], rgb: &[u8]) -> Self {
        assert_eq!(size[0] * size[1] * 3, rgb.len());
        let pixels = rgb
            .chunks_exact(3)
            .map(|p| Color32::from_rgb(p[0], p[1], p[2]))
            .collect();
        Self { size, pixels }
    }
}

fn resize_image_for_ui(
    (image_width, image_height): (f32, f32),
    (ui_width, ui_height): (f32, f32)
) -> (f32, f32) {
    if (image_width / image_height) > (ui_width / ui_height) {
        (ui_width, (ui_width * image_height) / image_width )
    } else if (image_width / image_height) < (ui_width / ui_height) {
        ((ui_height * image_width) / image_height, ui_height)
    } else if (image_width / image_height) == (ui_width / ui_height) {
        (ui_width, ui_height)
    } else {
        // Case should never happen. If requires an else block.
        (ui_width, ui_height)
    }
}
