use crate::subs::runnable::RunnableSubcommand;
use anyhow::Result;
use clap::Parser;
use solhat::calibrationframe::CalibrationImage;
use solhat::calibrationframe::ComputeMethod;
use solhat::context::*;
use solhat::drizzle::Scale;
use solhat::target::Target;
use solhat::threshtest::compute_threshtest_image;

pb_create_spinner!();

#[derive(Parser)]
#[command(author, version, about = "Compute a threshold test frame", long_about = None)]
pub struct ThreshTest {
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

    #[clap(long, short, help = "Object detection threshold")]
    threshold: Option<f64>,
}

#[async_trait::async_trait]
impl RunnableSubcommand for ThreshTest {
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

        let context = ProcessContext::create_with_calibration_frames(
            &ProcessParameters {
                input_files: self.input_files.clone(),
                obj_detection_threshold: self.threshold.unwrap_or(5000.0),
                obs_latitude: 0.0,
                obs_longitude: 0.0,
                target: Target::Sun,
                crop_width: None,
                crop_height: None,
                max_frames: None,
                min_sigma: None,
                max_sigma: None,
                top_percentage: None,
                drizzle_scale: Scale::Scale1_0,
                initial_rotation: 0.0,
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

        let first_frame = context.frame_records[0].get_calibrated_frame(&context)?;
        let result = compute_threshtest_image(
            &first_frame.buffer,
            context.parameters.obj_detection_threshold as f32,
        );

        result.save(&self.output)?;

        pb_done!();
        Ok(())
    }
}
