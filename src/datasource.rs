use anyhow::Result;
use sciimg::image;

use crate::timestamp;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ColorFormatId {
    Mono = 0,
    BayerRggb = 8,
    BayerGrbg = 9,
    BayerGbrg = 10,
    BayerBggr = 11,
    BayerCyym = 16,
    BayerYcmy = 17,
    BayerYmcy = 18,
    BayerMyyc = 19,
    Rgb = 100,
    Bgr = 101,
}

impl ColorFormatId {
    pub fn from_i32(v: i32) -> ColorFormatId {
        match v {
            0 => ColorFormatId::Mono,
            8 => ColorFormatId::BayerRggb,
            9 => ColorFormatId::BayerGrbg,
            10 => ColorFormatId::BayerGbrg,
            11 => ColorFormatId::BayerBggr,
            16 => ColorFormatId::BayerCyym,
            17 => ColorFormatId::BayerYcmy,
            18 => ColorFormatId::BayerYmcy,
            19 => ColorFormatId::BayerMyyc,
            100 => ColorFormatId::Rgb,
            101 => ColorFormatId::Bgr,
            _ => panic!("Invalid color format enum value: {}", v),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataFrame {
    pub buffer: image::Image,
    pub timestamp: timestamp::TimeStamp,
}

pub trait DataSource {
    fn color_id(&self) -> ColorFormatId;
    fn file_id(&self) -> String;
    fn image_width(&self) -> usize;
    fn image_height(&self) -> usize;
    fn pixel_depth(&self) -> usize;
    fn frame_count(&self) -> usize;
    fn observer(&self) -> String;
    fn instrument(&self) -> String;
    fn telescope(&self) -> String;
    fn date_time(&self) -> timestamp::TimeStamp;
    fn date_time_utc(&self) -> timestamp::TimeStamp;
    fn total_file_size(&self) -> usize;
    fn get_frame(&self, frame_num: usize) -> Result<DataFrame>;
    fn get_frame_timestamp(&self, frame_num: usize) -> Result<timestamp::TimeStamp>;

    fn source_file(&self) -> String;

    fn open(path: &str) -> Result<Self>
    where
        Self: Sized;

    fn validate(&self) -> Result<()>;

    fn print_header_details(&self);
}

#[allow(dead_code)]
pub type ImageDataSource = dyn DataSource + Send + Sync + 'static;
