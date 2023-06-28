#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use anyhow::{anyhow, Result};
use eframe::egui;
use solhat::anaysis::frame_sigma_analysis;
use solhat::calibrationframe::CalibrationImage;
use solhat::calibrationframe::ComputeMethod;
use solhat::context::*;
use solhat::drizzle::Scale;
use solhat::limiting::frame_limit_determinate;
use solhat::offsetting::frame_offset_analysis;
use solhat::rotation::frame_rotation_analysis;
use solhat::stacking::process_frame_stacking;
use solhat::target::Target;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::task::yield_now;

#[macro_use]
extern crate stump;

#[macro_use]
extern crate lazy_static;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    stump::set_min_log_level(stump::LogEntryLevel::DEBUG);
    info!("Starting SolHat-UI");
    let options = eframe::NativeOptions {
        ..Default::default()
    };
    eframe::run_native("SolHat", options, Box::new(|_cc| Box::<SolHat>::default()))
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
        }
    }
}

impl eframe::App for SolHat {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
    }
}

impl SolHat {
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

    fn inputs_frame_contents(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("inputs_3x3_lights")
            .num_columns(3)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                // Light Frames
                ui.label("Light:");
                ui.monospace(&self.light.clone().unwrap_or("".to_owned()));
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Light")
                        .add_filter("SER", &vec!["ser"])
                        .pick_file()
                    {
                        self.light = Some(path.display().to_string());

                        // If the output directory isn't yet set, we'll set
                        // it as the parent directory containing the file selected here.
                        if self.output_dir.is_none() {
                            self.output_dir = Some(path.parent().unwrap().display().to_string())
                        }
                    }
                }
                ui.end_row();

                // Darks
                ui.label("Dark:");
                ui.monospace(&self.dark.clone().unwrap_or("".to_owned()));
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Dark")
                        .add_filter("SER", &vec!["ser"])
                        .pick_file()
                    {
                        self.dark = Some(path.display().to_string());
                    }
                }
                ui.end_row();

                // Flats
                ui.label("Flat:");
                ui.monospace(&self.flat.clone().unwrap_or("".to_owned()));
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Flat")
                        .add_filter("SER", &vec!["ser"])
                        .pick_file()
                    {
                        self.flat = Some(path.display().to_string());
                    }
                }
                ui.end_row();

                // Dark Flat
                ui.label("Dark Flat:");
                ui.monospace(&self.darkflat.clone().unwrap_or("".to_owned()));
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Dark Flat")
                        .add_filter("SER", &vec!["ser"])
                        .pick_file()
                    {
                        self.darkflat = Some(path.display().to_string());
                    }
                }
                ui.end_row();

                // Bias
                ui.label("Bias:");
                ui.monospace(&self.bias.clone().unwrap_or("".to_owned()));
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Bias")
                        .add_filter("SER", &vec!["ser"])
                        .pick_file()
                    {
                        self.bias = Some(path.display().to_string());
                    }
                }
                ui.end_row();

                // Hot Pixel Map
                ui.label("Hot Pixel map:");
                ui.monospace(&self.hot_pixel_map.clone().unwrap_or("".to_owned()));
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Hot Pixel Map")
                        .add_filter("toml", &vec!["toml"])
                        .pick_file()
                    {
                        self.hot_pixel_map = Some(path.display().to_string());
                    }
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
        } = self;

        ui.label("Observer Latitude:");
        ui.add(egui::DragValue::new(obs_latitude).speed(1.0));
        ui.end_row();

        ui.label("Observer Longitude:");
        ui.add(egui::DragValue::new(obs_longitude).speed(1.0));
        ui.end_row();

        ui.label("Target:");
        egui::ComboBox::from_label("Target")
            .selected_text(format!("{:?}", target))
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
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
        } = self;

        ui.label("Object Detection Threshold:");
        ui.add(egui::DragValue::new(obj_detection_threshold).speed(1.0));
        ui.end_row();

        ui.label("Drizzle:");
        egui::ComboBox::from_label("Drizzle")
            .selected_text(format!("{:?}", drizzle_scale))
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
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

        let freetext = if self.freetext.len() > 0 {
            format!("_{}", self.freetext)
        } else {
            "".to_owned()
        };

        let drizzle = match self.drizzle_scale {
            Scale::Scale1_0 => "".to_owned(),
            _ => format!("_{}", self.drizzle_scale.to_string()),
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
            yield_now()
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

async fn run_async(params: ProcessParameters, output_filename: &PathBuf) -> Result<()> {
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
