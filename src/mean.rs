use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use rayon::prelude::*;
use sciimg::{enums::ImageMode, image};

use crate::datasource::{ColorFormatId, DataSource};

pub fn build_mean_buffer<F: DataSource + Send + Sync + 'static>(
    ser_file: &F,
) -> Result<image::Image> {
    let num_bands = match ser_file.color_id() {
        ColorFormatId::Mono => 1,
        _ => 3,
    };

    image::Image::new_with_bands(
        ser_file.image_width(),
        ser_file.image_height(),
        num_bands,
        match ser_file.pixel_depth() {
            8 => ImageMode::U8BIT,
            _ => ImageMode::U16BIT,
        },
    )
}

// Computes a simple mean stack of frames across a list of ser files.
pub fn compute_mean<F: DataSource + Send + Sync + 'static>(
    ser_file: &F,
    _skip_glitch_frames: bool,
) -> Result<image::Image> {
    let mut mean_buffer = build_mean_buffer(ser_file).unwrap();
    let buffer_mtx = Arc::new(Mutex::new(&mut mean_buffer));

    let cnt_mtx = Arc::new(Mutex::new(0));

    (0..ser_file.frame_count()).into_par_iter().for_each(|i| {
        let frame = ser_file.get_frame(i).expect("Failed to load image frame");
        // TODO: Add glitch frame detection

        buffer_mtx.lock().unwrap().add(&frame.buffer);

        let mut count = cnt_mtx.lock().unwrap();
        *count += 1;
    });

    let cnt = cnt_mtx.lock().unwrap();

    if *cnt > 0 {
        for band in 0..mean_buffer.num_bands() {
            mean_buffer.apply_weight_on_band(1.0 / *cnt as f32, band);
        }
        let (framemin, framemax) = mean_buffer.get_min_max_all_channel();
        vprintln!(
            "    Stack Min/Max : {}, {} ({} images)",
            framemin,
            framemax,
            cnt
        );
        Ok(mean_buffer)
    } else {
        eprintln!("No files used");
        Err(anyhow!("No files used"))
    }
}
