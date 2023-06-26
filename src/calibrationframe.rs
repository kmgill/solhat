use crate::mean;
use anyhow::Result;
use sciimg::prelude::*;
use std::{ffi::OsStr, path::Path};

pub struct CalibrationImage {
    pub image: Option<Image>,
}

fn create_mean_from_ser(ser_file_path: &str) -> Result<Image> {
    let input_files: Vec<&str> = vec![ser_file_path];
    let mean_stack = mean::compute_mean(&input_files, true)?;
    Ok(mean_stack)
}

impl CalibrationImage {
    pub fn new_from_file(file_path: &str) -> Result<Self> {
        if let Some(extension) = Path::new(file_path).extension().and_then(OsStr::to_str) {
            match extension.to_string().to_uppercase().as_str() {
                "SER" => CalibrationImage::new_from_ser(file_path),
                _ => CalibrationImage::new_from_image(file_path),
            }
        } else {
            Err(anyhow!("Unable to determine file type"))
        }
    }

    pub fn new_empty() -> Self {
        CalibrationImage { image: None }
    }

    pub fn new_from_ser(ser_path: &str) -> Result<Self> {
        let image = create_mean_from_ser(ser_path)?;
        Ok(CalibrationImage { image: Some(image) })
    }

    pub fn new_from_image(img_path: &str) -> Result<Self> {
        Ok(CalibrationImage {
            image: Some(Image::open(img_path)?),
        })
    }

    pub fn new_black(width: usize, height: usize, num_bands: usize) -> Result<Self> {
        Ok(CalibrationImage {
            image: Some(Image::new_with_bands_and_fill(
                width,
                height,
                num_bands,
                ImageMode::U16BIT,
                0.0,
            )?),
        })
    }

    pub fn new_as_mean_of_image(image: &Image) -> Result<Self> {
        let mean = image.get_band(0).mean() as f32;
        Ok(CalibrationImage {
            image: Some(Image::new_with_bands_and_fill(
                image.width,
                image.height,
                image.num_bands(),
                ImageMode::U16BIT,
                mean,
            )?),
        })
    }
}
