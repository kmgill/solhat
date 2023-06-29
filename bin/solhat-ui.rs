#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// use egui::panel::Side;
// use egui::Image;
// use egui::Vec2;
// use egui_extras::Size;
// use egui_extras::StripBuilder;
use anyhow::{anyhow, Result};
use eframe::egui;
use egui::Pos2;
use egui_extras::RetainedImage;
use epaint::image::ColorImage;
use epaint::Vec2;
use itertools::iproduct;
use serde::{Deserialize, Serialize};
use solhat::anaysis::frame_sigma_analysis;
use solhat::calibrationframe::CalibrationImage;
use solhat::calibrationframe::ComputeMethod;
use solhat::context::*;
use solhat::drizzle::Scale;
use solhat::limiting::frame_limit_determinate;
use solhat::offsetting::frame_offset_analysis;
use solhat::rotation::frame_rotation_analysis;
use solhat::ser::SerFile;
use solhat::ser::SerFrame;
use solhat::stacking::process_frame_stacking;
use solhat::target::Target;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

#[macro_use]
extern crate stump;

#[macro_use]
extern crate lazy_static;

// https://github.com/emilk/egui/discussions/1574
pub(crate) fn load_icon() -> eframe::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("../assets/solhat_icon_32x32.png");
        let image = image::load_from_memory(icon)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    eframe::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

enum TaskStatus {
    TaskPercentage(String, usize, usize),
}

#[derive(Default)]
struct TaskStatusContainer {
    status: Option<TaskStatus>,
}

lazy_static! {
    static ref TASK_STATUS_QUEUE: Arc<Mutex<TaskStatusContainer>> =
        Arc::new(Mutex::new(TaskStatusContainer::default()));
}

#[derive(Default, Deserialize, Serialize)]
struct WindowState {
    last_opened_folder: Option<PathBuf>,
    window_pos_x: usize,
    window_pos_y: usize,
    window_width: usize,
    window_height: usize,
    fullscreen: bool,
    theme: String,
}

#[derive(Deserialize, Serialize)]
struct SolHat {
    light: Option<String>,
    dark: Option<String>,
    flat: Option<String>,
    darkflat: Option<String>,
    bias: Option<String>,
    hot_pixel_map: Option<String>,
    output_dir: Option<String>,
    freetext: String,
    obs_latitude: f64,
    obs_longitude: f64,
    target: Target,
    obj_detection_threshold: f64,
    drizzle_scale: Scale,
    max_frames: usize,
    min_sigma: f64,
    max_sigma: f64,
    top_percentage: f64,
    state: WindowState,

    #[serde(skip_serializing, skip_deserializing)]
    thumbnail_main: Option<RetainedImage>,
}

fn ser_frame_to_retained_image(ser_frame: &SerFrame) -> RetainedImage {
    let mut copied = ser_frame.buffer.clone();
    let size: [usize; 2] = [copied.width as _, copied.height as _];
    copied.normalize_to_8bit();
    let mut rgb: Vec<u8> = Vec::with_capacity(copied.height * copied.width * 3);
    iproduct!(0..copied.height, 0..copied.width).for_each(|(y, x)| {
        let (r, g, b) = if copied.num_bands() == 1 {
            (
                copied.get_band(0).get(x, y),
                copied.get_band(0).get(x, y),
                copied.get_band(0).get(x, y),
            )
        } else {
            (
                copied.get_band(0).get(x, y),
                copied.get_band(1).get(x, y),
                copied.get_band(2).get(x, y),
            )
        };
        rgb.push(r as u8);
        rgb.push(g as u8);
        rgb.push(b as u8);
    });
    RetainedImage::from_color_image("thumbnail_main", ColorImage::from_rgb(size, &rgb))
}

impl WindowState {
    pub fn get_last_opened_folder(&self) -> PathBuf {
        if self.last_opened_folder.is_some() {
            self.last_opened_folder.to_owned().unwrap()
        } else {
            std::env::current_dir().unwrap()
        }
    }

    pub fn update_last_opened_folder(&mut self, path: &Path) {
        info!("Last opened path: {:?}", path);
        self.last_opened_folder = if path.is_file() {
            Some(path.parent().unwrap().to_path_buf())
        } else {
            Some(path.to_path_buf())
        };
    }

    pub fn update_from_window_info(&mut self, _ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(position) = frame.info().window_info.position {
            self.window_pos_x = position.x as usize;
            self.window_pos_y = position.y as usize;
        }

        let dimension = frame.info().window_info.size;
        self.window_width = dimension.x as usize;
        self.window_height = dimension.y as usize;

        self.fullscreen = frame.info().window_info.fullscreen;
    }
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    stump::set_min_log_level(stump::LogEntryLevel::DEBUG);
    info!("Starting SolHat-UI");

    let mut options = eframe::NativeOptions {
        icon_data: Some(load_icon()),
        initial_window_size: Some(Vec2 { x: 885.0, y: 650.0 }),
        min_window_size: Some(Vec2 { x: 885.0, y: 650.0 }),
        resizable: true,
        transparent: true,
        vsync: true,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        ..Default::default()
    };

    // If the config file (literally a serialized version of the last run window state) errors on read
    // or doesn't exist, we'll just ignore it and start from scratch.
    let solhat = if let Ok(solhat) = SolHat::load_from_userhome() {
        options.initial_window_pos = Some(Pos2::new(
            solhat.state.window_pos_x as f32,
            solhat.state.window_pos_y as f32,
        ));
        // This don't work on Linux (Fedora KDE). Windows keep growing...Likely
        // related to egui's insistence on 1.5x UI scale?
        // options.initial_window_size = Some(Vec2::new(
        //     solhat.state.window_width as f32,
        //     solhat.state.window_height as f32,
        // ));
        Box::new(solhat)
    } else {
        options.centered = true;
        Box::<SolHat>::default()
    };

    eframe::run_native("SolHat", options, Box::new(|_cc| solhat))
}

impl Default for SolHat {
    fn default() -> Self {
        Self {
            light: None,
            dark: None,
            flat: None,
            darkflat: None,
            bias: None,
            output_dir: None,
            freetext: "v1".to_owned(),
            obs_latitude: 34.0,
            obs_longitude: -118.0,
            target: Target::Sun,
            drizzle_scale: Scale::Scale1_0,
            obj_detection_threshold: 20000.0,
            hot_pixel_map: None,
            max_frames: 5000,
            min_sigma: 0.0,
            max_sigma: 1000.0,
            top_percentage: 100.0,
            state: WindowState::default(),
            thumbnail_main: None,
        }
    }
}

impl eframe::App for SolHat {
    fn on_close_event(&mut self) -> bool {
        self.save_to_userhome();
        true
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.enforce_value_bounds();
        self.state.update_from_window_info(ctx, frame);

        self.on_update(ctx, frame);
    }
}

impl SolHat {
    pub fn load_from_userhome() -> Result<Self> {
        let config_file_path = dirs::home_dir().unwrap().join(".solhat/config.toml");
        if config_file_path.exists() {
            info!(
                "Window state config file exists at path: {:?}",
                config_file_path
            );
            let t = std::fs::read_to_string(config_file_path)?;
            Ok(toml::from_str(&t)?)
        } else {
            warn!("Window state config file does not exist. Will be created on exit");
            Err(anyhow!("Config file does not exist"))
        }
    }

    pub fn save_to_userhome(&self) {
        let toml_str = toml::to_string(&self).unwrap();
        let solhat_config_dir = dirs::home_dir().unwrap().join(".solhat/");
        if !solhat_config_dir.exists() {
            fs::create_dir(&solhat_config_dir).expect("Failed to create config directory");
        }
        let config_file_path = solhat_config_dir.join("config.toml");
        let mut f = File::create(config_file_path).expect("Failed to create config file");
        f.write_all(toml_str.as_bytes())
            .expect("Failed to write to config file");
        debug!("{}", toml_str);
    }

    fn enforce_value_bounds(&mut self) {
        if self.obs_latitude > 90.0 {
            self.obs_latitude = 90.0; // Hello North Pole!
        } else if self.obs_latitude < -90.0 {
            self.obs_latitude = -90.0; // Hello South Pole!
        }

        // Longitude -180 through 180 where -180 is west.
        if self.obs_longitude > 180.0 {
            self.obs_longitude = 180.0;
        } else if self.obs_longitude < -180.0 {
            self.obs_longitude = -180.0;
        }

        if self.top_percentage > 100.0 {
            self.top_percentage = 100.0;
        } else if self.top_percentage < 1.0 {
            self.top_percentage = 1.0;
        }

        if self.obj_detection_threshold < 1.0 {
            self.obj_detection_threshold = 1.0;
        }

        if self.min_sigma > self.max_sigma {
            self.min_sigma = self.max_sigma; // Depends on which value is currently being modified
        }

        if self.max_sigma < self.min_sigma {
            self.max_sigma = self.min_sigma; // Depends on which value is currently being modified,
                                             // though max_sigma can drive down min_sigma to zero.
        }

        if self.min_sigma < 0.0 {
            self.min_sigma = 0.0;
        }

        if self.max_sigma < 0.0 {
            self.max_sigma = 0.0;
        }
    }

    #[allow(dead_code)]
    fn load_thumbnail(&mut self, force: bool) {
        if let Some(light_path) = &self.light {
            if self.thumbnail_main.is_none() || force {
                let ser_file = SerFile::load_ser(light_path).unwrap();
                let first_image = ser_file.get_frame(0).unwrap();
                self.thumbnail_main = Some(ser_frame_to_retained_image(&first_image));
            }
        } else {
            self.thumbnail_main = None;
        }
    }

    fn on_update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(1.0);
        // self.load_thumbnail(false);
        self.enforce_value_bounds();
        self.state.update_from_window_info(ctx, frame);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.separator();
            });

            ui.heading("Inputs");
            egui::Grid::new("process_grid_inputs")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    self.inputs_frame_contents(ui);
                });
            ui.separator();

            ui.heading("Output");
            egui::Grid::new("process_grid_outputs")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    self.outputs_frame_contents(ui);
                });
            ui.separator();

            ui.heading("Observation");
            egui::Grid::new("process_grid_observation")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    self.observation_frame_contents(ui);
                });
            ui.separator();

            ui.heading("Process Options");
            egui::Grid::new("process_grid_options")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    self.options_frame_contents(ui);
                });
            ui.separator();

            let proc_status = TASK_STATUS_QUEUE.lock().unwrap();

            match &proc_status.status {
                Some(TaskStatus::TaskPercentage(task_name, len, cnt)) => {
                    ui.vertical_centered(|ui| {
                        ui.monospace(task_name);
                        let pct = if *len > 0 {
                            *cnt as f32 / *len as f32
                        } else {
                            0.0
                        };
                        ui.add(egui::ProgressBar::new(pct).animate(true).show_percentage());
                        //ui.spinner();
                    });
                }
                None => {
                    ui.vertical_centered(|ui| {
                        ui.add_enabled_ui(self.enable_start(), |ui| {
                            if ui.button("START").clicked() {
                                let output_filename = self.assemble_output_filename().unwrap();
                                self.run(output_filename);
                                // Do STUFF!
                            }
                        });
                    });
                }
            }

            ui.separator();

            ui.vertical_centered(|ui| {
                ui.hyperlink("https://github.com/kmgill/solhat");
            });
        });

        // egui::SidePanel::new(Side::Right, "thumbnail-main")
        //     .resizable(false)
        //     .exact_width(500.0)
        //     .show(ctx, |ui| {
        //         if let Some(thumb) = &self.thumbnail_main {
        //             ui.add(
        //                 egui::Image::new(thumb.texture_id(ctx), thumb.size_vec2())
        //                     .rotate(45.0_f32.to_radians(), egui::Vec2::splat(0.5)),
        //             );
        //         }
        //     });
    }

    fn outputs_frame_contents(&mut self, ui: &mut egui::Ui) {
        // Light Frames
        ui.label("Output Folder:");
        ui.horizontal(|ui| {
            if let Some(output_dir) = &self.output_dir {
                ui.monospace(output_dir);
            }
            if ui.button("Open folder...").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.output_dir = Some(path.display().to_string());
                }
            }
        });
        ui.end_row();

        if let Ok(output_filename) = self.assemble_output_filename() {
            ui.label("Output Filename:");
            ui.monospace(output_filename.to_string_lossy().as_ref());
        }
    }

    fn truncate_to(s: &str, max_len: usize) -> String {
        if s.len() < max_len {
            s.to_owned()
        } else {
            let t: String = "...".to_owned() + &s[(s.len() - max_len + 3)..];
            // let t: String = s[0..(max_len - 3)].to_owned() + "...";
            t
        }
    }

    fn inputs_frame_contents(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("inputs_3x3_lights")
            .num_columns(4)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                // Light Frames
                ui.label("Light:");
                ui.monospace(&SolHat::truncate_to(
                    &self.light.clone().unwrap_or("".to_owned()),
                    80,
                ));
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Light")
                        .set_directory(self.state.get_last_opened_folder())
                        .add_filter("SER", &["ser"])
                        .pick_file()
                    {
                        self.light = Some(path.display().to_string());
                        self.state.update_last_opened_folder(&path);
                        // self.load_thumbnail(true);
                        // If the output directory isn't yet set, we'll set
                        // it as the parent directory containing the file selected here.
                        if self.output_dir.is_none() {
                            self.output_dir = Some(path.parent().unwrap().display().to_string())
                        }
                    }
                }
                if ui.button("Clear").clicked() {
                    self.light = None;
                }
                ui.end_row();

                // Darks
                ui.label("Dark:");
                ui.monospace(&SolHat::truncate_to(
                    &self.dark.clone().unwrap_or("".to_owned()),
                    80,
                ));
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Dark")
                        .set_directory(self.state.get_last_opened_folder())
                        .add_filter("SER", &["ser"])
                        .pick_file()
                    {
                        self.dark = Some(path.display().to_string());
                        self.state.update_last_opened_folder(&path);
                    }
                }
                if ui.button("Clear").clicked() {
                    self.dark = None;
                }
                ui.end_row();

                // Flats
                ui.label("Flat:");
                ui.monospace(&SolHat::truncate_to(
                    &self.flat.clone().unwrap_or("".to_owned()),
                    80,
                ));
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Flat")
                        .set_directory(self.state.get_last_opened_folder())
                        .add_filter("SER", &["ser"])
                        .pick_file()
                    {
                        self.flat = Some(path.display().to_string());
                        self.state.update_last_opened_folder(&path);
                    }
                }
                if ui.button("Clear").clicked() {
                    self.flat = None;
                }
                ui.end_row();

                // Dark Flat
                ui.label("Dark Flat:");
                ui.monospace(&SolHat::truncate_to(
                    &self.darkflat.clone().unwrap_or("".to_owned()),
                    80,
                ));
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Dark Flat")
                        .set_directory(self.state.get_last_opened_folder())
                        .add_filter("SER", &["ser"])
                        .pick_file()
                    {
                        self.darkflat = Some(path.display().to_string());
                        self.state.update_last_opened_folder(&path);
                    }
                }
                if ui.button("Clear").clicked() {
                    self.darkflat = None;
                }
                ui.end_row();

                // Bias
                ui.label("Bias:");
                ui.monospace(&SolHat::truncate_to(
                    &self.bias.clone().unwrap_or("".to_owned()),
                    80,
                ));
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Bias")
                        .set_directory(self.state.get_last_opened_folder())
                        .add_filter("SER", &["ser"])
                        .pick_file()
                    {
                        self.bias = Some(path.display().to_string());
                        self.state.update_last_opened_folder(&path);
                    }
                }
                if ui.button("Clear").clicked() {
                    self.bias = None;
                }
                ui.end_row();

                // Hot Pixel Map
                ui.label("Hot Pixel map:");
                ui.monospace(&SolHat::truncate_to(
                    &self.hot_pixel_map.clone().unwrap_or("".to_owned()),
                    80,
                ));
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Hot Pixel Map")
                        .set_directory(self.state.get_last_opened_folder())
                        .add_filter("toml", &["toml"])
                        .pick_file()
                    {
                        self.hot_pixel_map = Some(path.display().to_string());
                        self.state.update_last_opened_folder(&path);
                    }
                }
                if ui.button("Clear").clicked() {
                    self.hot_pixel_map = None;
                }
                ui.end_row();
            });
        ui.end_row();
    }

    fn observation_frame_contents(&mut self, ui: &mut egui::Ui) {
        let Self {
            light: _,
            dark: _,
            flat: _,
            darkflat: _,
            bias: _,
            output_dir: _,
            freetext: _,
            obs_latitude,
            obs_longitude,
            target,
            obj_detection_threshold: _,
            drizzle_scale: _,
            hot_pixel_map: _,
            max_frames: _,
            min_sigma: _,
            max_sigma: _,
            top_percentage: _,
            state: _,
            thumbnail_main: _,
        } = self;

        ui.label("Observer Latitude:");
        ui.add(
            egui::DragValue::new(obs_latitude)
                .min_decimals(1)
                .max_decimals(4)
                .speed(1.0),
        );
        ui.end_row();

        ui.label("Observer Longitude:");
        ui.add(
            egui::DragValue::new(obs_longitude)
                .min_decimals(1)
                .max_decimals(4)
                .speed(1.0),
        );
        ui.end_row();

        ui.label("Target:");
        ui.horizontal(|ui| {
            ui.selectable_value(target, Target::Sun, "Sun");
            ui.selectable_value(target, Target::Moon, "Moon");
        });
        ui.end_row();
    }

    fn options_frame_contents(&mut self, ui: &mut egui::Ui) {
        let Self {
            light: _,
            dark: _,
            flat: _,
            darkflat: _,
            bias: _,
            output_dir: _,
            freetext,
            obs_latitude: _,
            obs_longitude: _,
            target: _,
            drizzle_scale,
            hot_pixel_map: _,
            max_frames,
            min_sigma,
            max_sigma,
            obj_detection_threshold,
            top_percentage,
            state: _,
            thumbnail_main: _,
        } = self;

        ui.label("Object Detection Threshold:");
        ui.add(egui::DragValue::new(obj_detection_threshold).speed(10.0));
        ui.end_row();

        ui.label("Drizzle:");
        ui.horizontal(|ui| {
            ui.selectable_value(drizzle_scale, Scale::Scale1_0, "None");
            ui.selectable_value(drizzle_scale, Scale::Scale1_5, "1.5x");
            ui.selectable_value(drizzle_scale, Scale::Scale2_0, "2.0x");
            ui.selectable_value(drizzle_scale, Scale::Scale3_0, "3.0x");
        });
        ui.end_row();

        ui.label("Use Maximum Frames:");
        ui.add(egui::DragValue::new(max_frames).speed(10.0));
        ui.end_row();

        ui.label("Minimum Sigma:");
        ui.add(egui::DragValue::new(min_sigma).speed(1.0));
        ui.end_row();

        ui.label("Maximum Sigma:");
        ui.add(egui::DragValue::new(max_sigma).speed(1.0));
        ui.end_row();

        ui.label("Include Top Percentage:");
        ui.add(egui::DragValue::new(top_percentage).speed(1.0));
        ui.end_row();

        ui.label("Filename Free Text:");
        ui.add(egui::TextEdit::singleline(freetext).hint_text("Write something here"));
        ui.end_row();
    }

    fn assemble_output_filename(&self) -> Result<PathBuf> {
        let output_dir = if let Some(output_dir) = &self.output_dir {
            output_dir
        } else {
            return Err(anyhow!("Output directory not set"));
        };

        let base_filename = if let Some(input_file) = &self.light {
            Path::new(Path::new(input_file).file_name().unwrap())
                .file_stem()
                .unwrap()
        } else {
            return Err(anyhow!("Input light file not provided"));
        };

        let freetext = if !self.freetext.is_empty() {
            format!("_{}", self.freetext)
        } else {
            "".to_owned()
        };

        let drizzle = match self.drizzle_scale {
            Scale::Scale1_0 => "".to_owned(),
            _ => format!(
                "_{}",
                self.drizzle_scale.to_string().replace([' ', '.'], "")
            ),
        };

        let output_filename = format!(
            "{}_{:?}{}{}.tif",
            base_filename.to_string_lossy().as_ref(),
            self.target,
            drizzle,
            freetext
        );
        let output_path: PathBuf = Path::new(output_dir).join(output_filename);
        Ok(output_path)
    }

    fn to_parameters(&self) -> ProcessParameters {
        ProcessParameters {
            input_files: if let Some(light) = &self.light {
                vec![light.to_owned()]
            } else {
                vec![]
            },
            obj_detection_threshold: self.obj_detection_threshold,
            obs_latitude: self.obs_latitude,
            obs_longitude: self.obs_longitude,
            target: self.target,
            crop_width: None,
            crop_height: None,
            max_frames: Some(self.max_frames),
            min_sigma: Some(self.min_sigma),
            max_sigma: Some(self.max_sigma),
            top_percentage: Some(self.top_percentage),
            drizzle_scale: self.drizzle_scale,
            initial_rotation: 0.0,
            flat_inputs: self.flat.to_owned(),
            dark_inputs: self.dark.to_owned(),
            darkflat_inputs: self.darkflat.to_owned(),
            bias_inputs: self.bias.to_owned(),
            hot_pixel_map: self.hot_pixel_map.to_owned(),
        }
    }

    fn enable_start(&self) -> bool {
        self.light.is_some() && self.output_dir.is_some()
    }

    fn run(&mut self, output_filename: PathBuf) {
        let params = self.to_parameters();

        tokio::spawn(async move {
            {
                run_async(params, &output_filename).await.unwrap(); //.await.unwrap();
            }
        });
    }
}

fn increment_status() {
    let mut stat = TASK_STATUS_QUEUE.lock().unwrap();
    match &mut stat.status {
        Some(TaskStatus::TaskPercentage(name, len, val)) => {
            info!("Updating task status with value {}", val);
            stat.status = Some(TaskStatus::TaskPercentage(name.to_owned(), *len, *val + 1))
        }
        None => {}
    }
}

fn set_task_status(task_name: &str, len: usize, cnt: usize) {
    TASK_STATUS_QUEUE.lock().unwrap().status =
        Some(TaskStatus::TaskPercentage(task_name.to_owned(), len, cnt))
}

fn set_task_completed() {
    TASK_STATUS_QUEUE.lock().unwrap().status = None
}

async fn run_async(params: ProcessParameters, output_filename: &Path) -> Result<()> {
    set_task_status("Processing Master Flat", 2, 1);
    let master_flat = if let Some(inputs) = &params.flat_inputs {
        info!("Processing master flat...");
        CalibrationImage::new_from_file(inputs, ComputeMethod::Mean)?
    } else {
        CalibrationImage::new_empty()
    };

    set_task_status("Processing Master Dark Flat", 2, 1);
    let master_darkflat = if let Some(inputs) = &params.darkflat_inputs {
        info!("Processing master dark flat...");
        CalibrationImage::new_from_file(inputs, ComputeMethod::Mean)?
    } else {
        CalibrationImage::new_empty()
    };

    set_task_status("Processing Master Dark", 2, 1);
    let master_dark = if let Some(inputs) = &params.dark_inputs {
        info!("Processing master dark...");
        CalibrationImage::new_from_file(inputs, ComputeMethod::Mean)?
    } else {
        CalibrationImage::new_empty()
    };

    set_task_status("Processing Master Bias", 2, 1);
    let master_bias = if let Some(inputs) = &params.bias_inputs {
        info!("Processing master bias...");
        CalibrationImage::new_from_file(inputs, ComputeMethod::Mean)?
    } else {
        CalibrationImage::new_empty()
    };

    info!("Creating process context struct");
    let mut context = ProcessContext::create_with_calibration_frames(
        &params,
        master_flat,
        master_darkflat,
        master_dark,
        master_bias,
    )?;

    set_task_status("Frame Sigma Analysis", context.frame_records.len(), 0);
    context.frame_records = frame_sigma_analysis(&context, |_fr| {
        increment_status();
        info!("frame_sigma_analysis(): Frame processed.")
    })?;

    set_task_status("Applying Frame Limits", context.frame_records.len(), 0);
    context.frame_records = frame_limit_determinate(&context, |_fr| {
        info!("frame_limit_determinate(): Frame processed.")
    })?;

    set_task_status(
        "Computing Parallactic Angle Rotations",
        context.frame_records.len(),
        0,
    );
    context.frame_records = frame_rotation_analysis(&context, |fr| {
        increment_status();
        info!(
            "Rotation for frame is {} degrees",
            fr.computed_rotation.to_degrees()
        );
    })?;

    set_task_status(
        "Computing Center-of-Mass Offsets",
        context.frame_records.len(),
        0,
    );
    context.frame_records = frame_offset_analysis(&context, |_fr| {
        increment_status();
        info!("frame_offset_analysis(): Frame processed.")
    })?;

    if context.frame_records.is_empty() {
        println!("Zero frames to stack. Cannot continue");
    } else {
        set_task_status("Stacking", context.frame_records.len(), 0);
        let drizzle_output = process_frame_stacking(&context, |_fr| {
            info!("process_frame_stacking(): Frame processed.");
            increment_status();
        })?;

        set_task_status("Finalizing", 2, 1);
        let mut stacked_buffer = drizzle_output.get_finalized().unwrap();

        // Let the user know some stuff...
        let (stackmin, stackmax) = stacked_buffer.get_min_max_all_channel();
        info!(
            "    Stack Min/Max : {}, {} ({} images)",
            stackmin,
            stackmax,
            context.frame_records.len()
        );
        stacked_buffer.normalize_to_16bit();
        info!(
            "Final image size: {}, {}",
            stacked_buffer.width, stacked_buffer.height
        );

        // Save finalized image to disk
        set_task_status("Saving", 2, 1);
        stacked_buffer.save(output_filename.to_string_lossy().as_ref())?;

        // The user will likely never see this actually appear on screen
        set_task_status("Done", 1, 1);
    }

    set_task_completed();

    Ok(())
}
