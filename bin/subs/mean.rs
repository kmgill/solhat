use crate::subs::runnable::RunnableSubcommand;
use anyhow::Result;
use clap::Parser;
use solhat::calibrationframe::CalibrationImage;
use solhat::calibrationframe::ComputeMethod;
use solhat::hotpixel;

pb_create_spinner!();

#[derive(Parser)]
#[command(author, version, about = "Create a calibration as the average of pixels", long_about = None)]
pub struct Mean {
    #[clap(long, short, help = "Input ser file")]
    input_files: String,

    #[clap(long, short, help = "Output image")]
    output: String,

    #[clap(long, short = 'p', help = "Hot pixel map")]
    hotpixelmap: Option<String>,
}

#[async_trait::async_trait]
impl RunnableSubcommand for Mean {
    async fn run(&self) -> Result<()> {
        pb_set_print!();
        let mut calimage = CalibrationImage::new_from_file(&self.input_files, ComputeMethod::Mean)?;

        if let Some(hpm_path) = &self.hotpixelmap {
            let hpm = hotpixel::load_hotpixel_map(hpm_path)?;
            let mask = hotpixel::create_hotpixel_mask(&hpm)?;
            calimage.image = Some(hotpixel::replace_hot_pixels(
                &mut calimage.image.unwrap(),
                &mask,
            ));
        }

        calimage.image.unwrap().save(&self.output)?;
        pb_done!();
        Ok(())
    }
}
