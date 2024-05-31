use std::fmt::Display;

use crate::point::Point;
use anyhow::{anyhow, Result};
use sciimg::imagebuffer::Offset;
use sciimg::matrix::Matrix;
use sciimg::prelude::*;
use sciimg::vector::Vector;
use serde::{Deserialize, Serialize};

fn round_f64(v: f64) -> f64 {
    (v * 100000.0).round() / 100000.0
}

#[derive(Debug, Copy, Clone, PartialEq, Default, Deserialize, Serialize)]
pub enum StackAlgorithm {
    #[default]
    Average,
    Median,
    Minimum,
}

impl StackAlgorithm {
    pub fn allow_parallel(self) -> bool {
        match self {
            StackAlgorithm::Average | Self::Minimum => true,
            StackAlgorithm::Median => false, // Current implementation will use WAY too much memory
        }
    }
}

#[derive(Debug, Clone)]
pub enum StackAlgorithmImpl {
    Average(AverageStackBuffer),
    Median(MedianStackBuffer),
    Minimum(MinimumStackBuffer),
}

/// Supported drizzle scalings
#[derive(Debug, Copy, Clone, PartialEq, Default, Deserialize, Serialize)]
pub enum Scale {
    #[default]
    Scale1_0, // No upscaling
    Scale1_5,
    Scale2_0,
    Scale3_0,
}

impl Scale {
    pub fn value(&self) -> f32 {
        match *self {
            Scale::Scale1_0 => 1.0,
            Scale::Scale1_5 => 1.5,
            Scale::Scale2_0 => 2.0,
            Scale::Scale3_0 => 3.0,
        }
    }
}

impl Scale {
    pub fn from(s: &str) -> Result<Scale> {
        match s {
            "1.0" => Ok(Scale::Scale1_0),
            "1.5" => Ok(Scale::Scale1_5),
            "2.0" => Ok(Scale::Scale2_0),
            "3.0" => Ok(Scale::Scale3_0),
            _ => Err(anyhow!(
                "Invalid drizze scale: {}. Valid options: 1.0, 1.5, 2.0, 3.0",
                s
            )),
        }
    }
}

impl Display for Scale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scale::Scale1_0 => write!(f, "Drizzle x1"),
            Scale::Scale1_5 => write!(f, "Drizzle x1.5"),
            Scale::Scale2_0 => write!(f, "Drizzle x2.0"),
            Scale::Scale3_0 => write!(f, "Drizzle x3.0"),
        }
    }
}

pub trait StackBuffer {
    fn new(width: usize, height: usize, num_bands: usize) -> Self;
    fn put(&mut self, x: usize, y: usize, values: &[f32; 3]);
    fn get(&self, x: usize, y: usize, band: usize) -> f32;
    fn get_finalized(&self) -> Result<Image>;
    fn num_bands(&self) -> usize;
    fn add_other(&mut self, other: &Self);
}

#[derive(Debug, Clone)]
pub struct MinimumStackBuffer {
    num_bands: usize,
    buffer: Image,
}

impl StackBuffer for MinimumStackBuffer {
    fn new(width: usize, height: usize, num_bands: usize) -> Self {
        MinimumStackBuffer {
            num_bands,
            buffer: Image::new_with_bands(width, height, num_bands, ImageMode::U16BIT)
                .expect("Failed to allocate rgbimage"),
        }
    }

    fn put(&mut self, x: usize, y: usize, values: &[f32; 3]) {
        (0..self.num_bands).for_each(|i| {
            let v = self.get(x, y, i);

            let use_v = if v > 0.0 { values[i].min(v) } else { values[i] };

            self.buffer.put(x, y, use_v, i);
        });
    }

    fn get(&self, x: usize, y: usize, band: usize) -> f32 {
        if band >= self.num_bands {
            0.0
        } else {
            self.buffer.get_band(band).get(x, y)
        }
    }

    fn get_finalized(&self) -> Result<Image> {
        Ok(self.buffer.clone())
    }

    fn num_bands(&self) -> usize {
        self.num_bands
    }

    fn add_other(&mut self, other: &Self) {
        for y in 0..self.buffer.height {
            for x in 0..self.buffer.width {
                for b in 0..self.num_bands {
                    let v = self.get(x, y, b);
                    let ov = other.get(x, y, b);

                    let fv = if v > 0.0 { v.min(ov) } else { ov };

                    self.buffer.put(x, y, fv, b);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AverageStackBuffer {
    num_bands: usize,
    buffer: Image,
    divisor: ImageBuffer,
}

impl StackBuffer for AverageStackBuffer {
    fn new(width: usize, height: usize, num_bands: usize) -> Self {
        AverageStackBuffer {
            num_bands,
            buffer: Image::new_with_bands(width, height, num_bands, ImageMode::U16BIT)
                .expect("Failed to allocate rgbimage"),
            divisor: ImageBuffer::new(width, height)
                .expect("Failed to create drizzle divisor buffer"),
        }
    }

    fn put(&mut self, x: usize, y: usize, values: &[f32; 3]) {
        (0..self.num_bands).for_each(|i| {
            let v = self.get(x, y, i);
            self.buffer.put(x, y, v + values[i], i);
        });
        self.divisor.put(x, y, 1.0);
    }

    fn get(&self, x: usize, y: usize, band: usize) -> f32 {
        if band >= self.num_bands {
            0.0
        } else {
            self.buffer.get_band(band).get(x, y)
        }
    }

    fn get_finalized(&self) -> Result<Image> {
        let mut final_buffer = self.buffer.clone();
        final_buffer.divide_from_each(&self.divisor);
        Ok(final_buffer)
    }

    fn num_bands(&self) -> usize {
        self.num_bands
    }

    fn add_other(&mut self, other: &Self) {
        self.buffer.add(&other.buffer);
        self.divisor.add_mut(&other.divisor);
    }
}

#[derive(Debug, Clone)]
struct VectorMatrix {
    width: usize,
    height: usize,
    matrix: Vec<Vec<f32>>,
}

impl VectorMatrix {
    pub fn new(width: usize, height: usize) -> Self {
        let l = width * height;
        let matrix = (0..l)
            .map(|_| Vec::with_capacity(200))
            .collect::<Vec<Vec<f32>>>();
        VectorMatrix {
            width,
            height,
            matrix,
        }
    }

    pub fn put(&mut self, x: usize, y: usize, value: f32) {
        let idx = y * self.width + x;
        if idx < self.matrix.len() {
            self.matrix[idx].push(value);
        }
    }

    pub fn get_median(&self, x: usize, y: usize) -> f32 {
        let idx = y * self.width + x;
        let mut s = self.matrix[idx].clone();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        s[s.len() / 2]
    }

    pub fn extend(&mut self, other: &VectorMatrix) {
        for i in 0..self.matrix.len() {
            self.matrix[i].extend(&other.matrix[i]);
        }
    }

    pub fn to_median_image(&self) -> ImageBuffer {
        let mut buffer =
            ImageBuffer::new_as_mode(self.width, self.height, ImageMode::U16BIT).unwrap();

        for y in 0..self.height {
            for x in 0..self.width {
                buffer.put(x, y, self.get_median(x, y));
            }
        }
        buffer
    }
}

#[derive(Debug, Clone)]
pub struct MedianStackBuffer {
    num_bands: usize,
    width: usize,
    height: usize,
    matrix: Vec<VectorMatrix>,
}

impl StackBuffer for MedianStackBuffer {
    fn new(width: usize, height: usize, num_bands: usize) -> Self {
        let m = (0..num_bands)
            .map(|_| VectorMatrix::new(width, height))
            .collect();

        MedianStackBuffer {
            num_bands,
            width,
            height,
            matrix: m,
        }
    }

    fn put(&mut self, x: usize, y: usize, values: &[f32; 3]) {
        for (b, value) in values.iter().enumerate().take(self.num_bands) {
            self.matrix[b].put(x, y, *value);
        }
    }

    fn get(&self, x: usize, y: usize, band: usize) -> f32 {
        self.matrix[band].get_median(x, y)
    }

    fn get_finalized(&self) -> Result<Image> {
        let mut image =
            Image::new_with_bands(self.width, self.height, self.num_bands, ImageMode::U16BIT)
                .unwrap();

        for b in 0..self.num_bands {
            image.set_band(&self.matrix[b].to_median_image(), b);
        }

        Ok(image)
    }

    fn num_bands(&self) -> usize {
        self.num_bands
    }

    fn add_other(&mut self, other: &Self) {
        for b in 0..self.num_bands {
            self.matrix[b].extend(&other.matrix[b]);
        }
    }
}

#[derive(Debug, Clone)]
pub struct BilinearDrizzle {
    in_width: usize,
    in_height: usize,
    out_width: usize,
    out_height: usize,
    buffer: StackAlgorithmImpl,
    frame_add_count: usize,
    horiz_offset: i32,
    vert_offset: i32,
}

impl BilinearDrizzle {
    pub fn new(
        in_width: usize,
        in_height: usize,
        scale: Scale,
        horiz_offset: i32,
        vert_offset: i32,
        stack_buffer: StackAlgorithmImpl,
    ) -> BilinearDrizzle {
        let out_width = (in_width as f32 * scale.value()).ceil() as usize;
        let out_height = (in_height as f32 * scale.value()).ceil() as usize;
        BilinearDrizzle {
            in_width,
            in_height,
            out_width,
            out_height,
            frame_add_count: 0,
            horiz_offset,
            vert_offset,
            buffer: stack_buffer,
        }
    }

    /// Convert an x/y point on the drizzle buffer to the respective point on the input buffer
    fn buffer_point_to_input_point(&self, out_x: usize, out_y: usize) -> Point {
        if out_x < self.out_width && out_y < self.out_height {
            let x = round_f64((out_x as f64 / self.out_width as f64) * self.in_width as f64);
            let y = round_f64((out_y as f64 / self.out_height as f64) * self.in_height as f64);

            Point {
                x: x as f32,
                y: y as f32,
                valid: (x < self.in_width as f64 && y < self.in_height as f64),
            }
        } else {
            Point {
                x: -1.0,
                y: -1.0,
                valid: false,
            }
        }
    }

    // Adds the image. Each pixel point will be transformed by the offset and rotation. Rotation is relative to
    // the center of mass.
    pub fn add_with_transform(
        &mut self,
        other: &Image,
        offset: &Offset,
        rotation: f64,
    ) -> Result<()> {
        info!(
            "Adding drizzle frame of offset {:?} and rotation {}",
            offset,
            rotation.to_degrees()
        );

        //let mut mtx = Matrix::identity();
        let mtx = Matrix::rotate(rotation, Axis::ZAxis);

        for y in 0..self.out_height as i32 {
            for x in 0..self.out_width as i32 {
                let mut in_pt = self.buffer_point_to_input_point(x as usize, y as usize);

                let mut pt_vec = Vector::new(
                    in_pt.x as f64 - (other.width / 2) as f64,
                    in_pt.y as f64 - (other.height / 2) as f64,
                    0.0,
                );

                pt_vec = mtx.multiply_vector(&pt_vec);

                in_pt.x = pt_vec.x as f32 + (other.width / 2) as f32;
                in_pt.y = pt_vec.y as f32 + (other.height / 2) as f32;

                in_pt.x -= offset.h;
                in_pt.y -= offset.v;

                let mut abc: [f32; 3] = [0.0, 0.0, 0.0];

                for (band, value) in abc.iter_mut().enumerate().take(other.num_bands()) {
                    if let Some(v) = in_pt.get_interpolated_color(other.get_band(band)) {
                        *value = v;
                    }
                }

                let x2 = x + self.horiz_offset;
                let y2 = y + self.vert_offset;

                if x2 >= self.out_width as i32 || y2 >= self.out_height as i32 || x2 < 0 || y2 < 0 {
                    continue;
                }

                match &mut self.buffer {
                    StackAlgorithmImpl::Average(sai) => {
                        if other.num_bands() == 1 && sai.num_bands() == 3 {
                            abc[1] = abc[0];
                            abc[2] = abc[0];
                        }

                        sai.put(x as usize, y as usize, &abc);
                    }
                    StackAlgorithmImpl::Median(sai) => {
                        if other.num_bands() == 1 && sai.num_bands() == 3 {
                            abc[1] = abc[0];
                            abc[2] = abc[0];
                        }

                        sai.put(x as usize, y as usize, &abc);
                    }
                    StackAlgorithmImpl::Minimum(sai) => {
                        if other.num_bands() == 1 && sai.num_bands() == 3 {
                            abc[1] = abc[0];
                            abc[2] = abc[0];
                        }

                        sai.put(x as usize, y as usize, &abc);
                    }
                };
            }
        }
        self.frame_add_count += 1;
        Ok(())
    }

    pub fn get_finalized(&self) -> Result<Image> {
        if self.frame_add_count == 0 {
            Err(anyhow!(
                "No frames have been added, cannot divide mean by zero"
            ))
        } else {
            // let mut final_buffer = self.buffer.clone();
            // final_buffer.divide_from_each(&self.divisor);
            // Ok(final_buffer)

            match &self.buffer {
                StackAlgorithmImpl::Average(sai) => sai.get_finalized(),
                StackAlgorithmImpl::Median(sai) => sai.get_finalized(),
                StackAlgorithmImpl::Minimum(sai) => sai.get_finalized(),
            }
        }
    }

    pub fn add_drizzle(&mut self, other: &BilinearDrizzle) -> Result<()> {
        if other.out_width != self.out_width {
            return Err(anyhow!("Buffer dimensions are different. Cannot merge"));
        }
        match &mut self.buffer {
            StackAlgorithmImpl::Average(sai) => {
                if let StackAlgorithmImpl::Average(sai_other) = &other.buffer {
                    sai.add_other(sai_other);
                }
            }
            StackAlgorithmImpl::Median(sai) => {
                if let StackAlgorithmImpl::Median(sai_other) = &other.buffer {
                    sai.add_other(sai_other);
                }
            }
            StackAlgorithmImpl::Minimum(sai) => {
                if let StackAlgorithmImpl::Minimum(sai_other) = &other.buffer {
                    sai.add_other(sai_other);
                }
            }
        }
        // self.buffer.add_other(&other.buffer);
        // self.buffer.add(&other.buffer);
        // self.divisor.add_mut(&other.divisor);
        self.frame_add_count += other.frame_add_count;

        Ok(())
    }
}
