use crate::subs::runnable::RunnableSubcommand;
use anyhow::Result;
use clap::Parser;
// use sciimg::prelude::*;
use solhat::calibrationframe::CalibrationImage;
use solhat::calibrationframe::ComputeMethod;
use solhat::context::*;
use solhat::drizzle::Scale;
use solhat::target::Target;

pb_create!();

#[derive(Parser)]
#[command(author, version, about = "Preprocess an observation", long_about = None)]
pub struct PreProcess {
    #[clap(long, short, help = "Input ser files")]
    input_files: Vec<String>,

    #[clap(long, short, help = "Flat frame file")]
    flat: Option<String>,

    #[clap(long, short, help = "Dark frame file")]
    dark: Option<String>,

    #[clap(long, short = 'D', help = "Dark Flat frame file")]
    darkflat: Option<String>,

    #[clap(long, short, help = "Bias frame file")]
    bias: Option<String>,

    #[clap(long, short, help = "Crop width")]
    width: Option<usize>,

    #[clap(long, short = 'H', help = "Crop height")]
    height: Option<usize>,

    #[clap(long, short, help = "Observer latitude", allow_hyphen_values(true))]
    latitude: f64,

    #[clap(
        long,
        short = 'L',
        help = "Observer longitude",
        allow_hyphen_values(true)
    )]
    longitude: f64,

    #[clap(long, short, help = "Object detection threshold")]
    threshold: Option<f64>,

    #[clap(long, short = 's', help = "Minimum sigma value")]
    minsigma: Option<f64>,

    #[clap(long, short = 'S', help = "Maximum sigma value")]
    maxsigma: Option<f64>,

    #[clap(
        long,
        short = 'I',
        help = "Force an initial rotation value",
        allow_hyphen_values(true)
    )]
    rotation: Option<f64>,

    #[clap(
        long,
        short = 'P',
        help = "Scale maximum value to percentage max possible (0-100)"
    )]
    percentofmax: Option<f64>,

    #[clap(long, short, help = "Number of frames (default=all)")]
    number_of_frames: Option<usize>,

    #[clap(long, short = 'T', help = "Target (Moon, Sun)")]
    target: Option<String>,

    #[clap(long, short = 'u', help = "Drizze upscale (1.5, 2.0, 3.0")]
    drizzle: Option<String>,

    #[clap(long, short = 'r', help = "Process report path")]
    report: Option<String>,
}

#[async_trait::async_trait]
impl RunnableSubcommand for PreProcess {
    async fn run(&self) -> Result<()> {
        pb_set_print!();

        let master_flat = if let Some(inputs) = &self.flat {
            CalibrationImage::new_from_file(inputs, ComputeMethod::Mean)?
        } else {
            CalibrationImage::new_empty()
        };

        let master_darkflat = if let Some(inputs) = &self.darkflat {
            CalibrationImage::new_from_file(inputs, ComputeMethod::Mean)?
        } else {
            CalibrationImage::new_empty()
        };

        let master_dark = if let Some(inputs) = &self.dark {
            CalibrationImage::new_from_file(inputs, ComputeMethod::Mean)?
        } else {
            CalibrationImage::new_empty()
        };

        let master_bias = if let Some(inputs) = &self.bias {
            CalibrationImage::new_from_file(inputs, ComputeMethod::Mean)?
        } else {
            CalibrationImage::new_empty()
        };

        let _context = ProcessContext::create_with_calibration_frames(
            &ProcessParameters {
                input_files: self.input_files.clone(),
                obj_detection_threshold: self.threshold.unwrap_or(5000.0),
                obs_latitude: self.latitude,
                obs_longitude: self.longitude,
                target: Target::from(&self.target.to_owned().unwrap_or("sun".to_owned()))?,
                crop_width: self.width,
                crop_height: self.height,
                max_frames: self.number_of_frames,
                min_sigma: self.minsigma,
                max_sigma: self.maxsigma,
                top_percentage: self.percentofmax,
                drizzle_scale: Scale::from(&self.drizzle.to_owned().unwrap_or("1.0".to_owned()))?,
                initial_rotation: self.rotation.unwrap_or(0.0),
                flat_inputs: self.flat.to_owned(),
                dark_inputs: self.dark.to_owned(),
                darkflat_inputs: self.darkflat.to_owned(),
                bias_inputs: self.bias.to_owned(),
            },
            master_flat,
            master_darkflat,
            master_dark,
            master_bias,
        )?;
        pb_done!();
        Ok(())
    }
}
