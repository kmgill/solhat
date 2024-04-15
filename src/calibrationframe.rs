use std::{ffi::OsStr, path::Path};

use anyhow::{Error, Result};
use sciimg::prelude::*;

use crate::datasource::DataSource;
use crate::mean;
use crate::median;
use crate::ser::SerFile;

#[derive(Debug, PartialEq, Eq)]
pub enum ComputeMethod {
    Mean,
    Median,
}

pub struct CalibrationImage {
    pub image: Option<Image>,
}

fn create_mean_from_ser<F: DataSource + Send + Sync + 'static>(ser_file: &F) -> Result<Image> {
    let mean_stack = mean::compute_mean(ser_file, true)?;
    Ok(mean_stack)
}

fn create_median_from_ser<F: DataSource + Send + Sync + 'static>(ser_file: &F) -> Result<Image> {
    median::compute_mean(ser_file)
}

impl CalibrationImage {
    pub fn new_from_file(file_path: &str, method: ComputeMethod) -> Result<Self> {
        if let Some(extension) = Path::new(file_path).extension().and_then(OsStr::to_str) {
            match extension.to_string().to_uppercase().as_str() {
                "SER" => CalibrationImage::new_from_ser(file_path, method),
                _ => CalibrationImage::new_from_image(file_path),
            }
        } else {
            Err(Error::msg("Unable to determine file type"))
        }
    }

    pub fn new_empty() -> Self {
        CalibrationImage { image: None }
    }

    pub fn new_from_ser(ser_path: &str, method: ComputeMethod) -> Result<Self> {
        let ser_file = SerFile::load_ser(ser_path)?;
        let image = match method {
            ComputeMethod::Mean => create_mean_from_ser(&ser_file)?,
            ComputeMethod::Median => create_median_from_ser(&ser_file)?,
        };
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
        let mean = image.get_band(0).mean();
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
