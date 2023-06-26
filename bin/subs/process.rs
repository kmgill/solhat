use crate::subs::runnable::RunnableSubcommand;
use anyhow::Result;
use clap::Parser;
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

pb_create!();

#[derive(Parser)]
#[command(author, version, about = "Process an observation", long_about = None)]
pub struct Process {
    #[clap(long, short, help = "Input ser files")]
    input_files: Vec<String>,

    #[clap(long, short, help = "Output image")]
    output: String,

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
impl RunnableSubcommand for Process {
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

        info!("Creating process context...");
        let mut context = ProcessContext::create_with_calibration_frames(
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

        // Calculate frame sigmas (quality)
        info!("Calculating frame sigma");
        pb_set_prefix!("Calculating Frame Sigma");
        pb_set_length!(context.frame_records.len());
        context.frame_records = frame_sigma_analysis(&context, |_fr| {
            pb_inc!();
        })?;

        // Limit frames based on rules.
        info!("Applying frame limits");
        pb_zero!();
        pb_set_prefix!("Applying Frame Limits");
        pb_set_length!(context.frame_records.len());
        context.frame_records = frame_limit_determinate(&context, |_fr| {
            pb_inc!();
        })?;

        // Compute parallactic angle rotations
        info!("Computing parallactic angle rotations");
        pb_zero!();
        pb_set_prefix!("Computing Parallactic Angle Frame Rotations");
        pb_set_length!(context.frame_records.len());
        context.frame_records = frame_rotation_analysis(&context, |fr| {
            info!(
                "Rotation for frame is {} degrees",
                fr.computed_rotation.to_degrees()
            );
            pb_inc!();
        })?;

        // Compute center-of-mass offsets for each frame
        info!("Computing center-of-mass offsets for frames");
        pb_zero!();
        pb_set_prefix!("Computing Center-of-Mass Offsets for Frames");
        pb_set_length!(context.frame_records.len());
        context.frame_records = frame_offset_analysis(&context, |_fr| {
            pb_inc!();
        })?;

        if context.frame_records.is_empty() {
            println!("Zero frames to stack. Cannot continue");
        } else {
            // Stacking
            info!("Stacking");
            pb_zero!();
            pb_set_prefix!("Stacking Frames");
            pb_set_length!(context.frame_records.len());
            let drizzle_output = process_frame_stacking(&context, |_fr| {
                pb_inc!();
            })?;

            info!("Finalizing and saving");
            let mut stacked_buffer = drizzle_output.get_finalized().unwrap();

            // Zero would indicate that the user did not ask for cropping since that's not a valid crop dimension anyway
            let crop_width = context.parameters.crop_width.unwrap_or(0);
            let crop_height = context.parameters.crop_height.unwrap_or(0);

            if crop_width > 0
                && crop_height > 0
                && crop_width <= stacked_buffer.width
                && crop_height <= stacked_buffer.height
            {
                // Scale the crop up by the drizzle scale factor
                let crop_width =
                    (crop_width as f32 * context.parameters.drizzle_scale.value()).round() as usize;
                let crop_height = (crop_height as f32 * context.parameters.drizzle_scale.value())
                    .round() as usize;

                info!(
                    "Cropping image to width/height: {} / {}",
                    crop_width, crop_height,
                );
                let x = (stacked_buffer.width - crop_width) / 2;
                let y = (stacked_buffer.height - crop_height) / 2;
                stacked_buffer.crop(x, y, crop_width, crop_height);
            }

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
            stacked_buffer.save(&self.output)?;
        }

        pb_done!();
        Ok(())
    }
}
