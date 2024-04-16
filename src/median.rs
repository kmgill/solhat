use anyhow::{anyhow, Result};
use sciimg::prelude::*;

use crate::datasource::ColorFormatId;
use crate::datasource::DataSource;

#[derive(Debug, Default)]
struct MedianBuffer {
    buffer: Vec<Vec<f32>>,
}

impl MedianBuffer {
    pub fn new(length: usize) -> Self {
        let mut v: Vec<Vec<f32>> = Vec::with_capacity(length);
        (0..length).for_each(|_| {
            v.push(vec![]);
        });

        MedianBuffer { buffer: v }
    }
    pub fn add_buffer(&mut self, other: &[f32]) -> Result<()> {
        if other.len() != self.buffer.len() {
            Err(anyhow!(
                "Other buffer is of a non-matching length: {} != {}",
                other.len(),
                self.buffer.len()
            ))
        } else {
            (0..self.buffer.len()).for_each(|i| {
                self.buffer[i].push(other[i]);
            });
            Ok(())
        }
    }
    pub fn get_median_vector(&mut self) -> Vec<f32> {
        let mut v: Vec<f32> = Vec::with_capacity(self.buffer.len());
        (0..self.buffer.len()).for_each(|i| {
            self.buffer[i].sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mv = self.buffer[i][self.buffer[i].len() / 2];
            v.push(mv);
        });
        v
    }
}

pub fn compute_mean<F: DataSource>(ser_file: &F) -> Result<Image> {
    let mut median_buffers = match ser_file.color_id() {
        ColorFormatId::Mono => vec![MedianBuffer::new(
            ser_file.image_width() * ser_file.image_height(),
        )],
        _ => vec![
            MedianBuffer::new(ser_file.image_width() * ser_file.image_height()),
            MedianBuffer::new(ser_file.image_width() * ser_file.image_height()),
            MedianBuffer::new(ser_file.image_width() * ser_file.image_height()),
        ],
    };

    iproduct!(0..ser_file.frame_count(), 0..median_buffers.len()).for_each(|(f, b)| {
        let frame = ser_file.get_frame(f).expect("Failed to load image frame");
        median_buffers[b]
            .add_buffer(&frame.buffer.get_band(0).buffer.to_vector())
            .expect("Failed to add band buffer to median");
    });

    Ok(match ser_file.color_id() {
        ColorFormatId::Mono => Image::new_from_buffer_mono(&ImageBuffer::from_vec(
            &median_buffers[0].get_median_vector(),
            ser_file.image_width(),
            ser_file.image_height(),
        )?)?,
        _ => Image::new_from_buffers_rgb(
            &ImageBuffer::from_vec(
                &median_buffers[0].get_median_vector(),
                ser_file.image_width(),
                ser_file.image_height(),
            )?,
            &ImageBuffer::from_vec(
                &median_buffers[0].get_median_vector(),
                ser_file.image_width(),
                ser_file.image_height(),
            )?,
            &ImageBuffer::from_vec(
                &median_buffers[0].get_median_vector(),
                ser_file.image_width(),
                ser_file.image_height(),
            )?,
            ImageMode::U16BIT,
        )?,
    })
}
