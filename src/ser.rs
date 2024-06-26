// Technical specification: http://www.grischa-hahn.homepage.t-online.de/astro/ser/SER%20Doc%20V3b.pdf

use anyhow::{Error, Result};
use sciimg::{binfilereader::*, debayer, enums::ImageMode, image, imagebuffer};

use crate::datasource::{ColorFormatId, DataFrame, DataSource};
use crate::timestamp;
use crate::timestamp::TimeStamp;

const HEADER_SIZE_BYTES: usize = 178;
const TIMESTAMP_SIZE_BYTES: usize = 8;

// Variable size of pixel_depth * image_width * image_height
// Frames block is frame_size * num_images
// Frames block starts off at byte 178
#[derive(Debug, Clone)]
pub struct SerFrame {
    pub buffer: image::Image,
    pub timestamp: timestamp::TimeStamp,
}

// Header is a fixed size of 178 bytes
// Optional trailer starts at num_images * pixel_depth * image_width * image_height
// Trailer size is 8 byte (i64) time stamps for each frame, size is 8 * num_images
pub struct SerFile {
    pub file_id: String,                     // 14 bytes
    pub camera_series_id: i32,               // 4 bytes
    pub color_id: ColorFormatId,             // 4 bytes
    pub image_width: usize,                  // 4 bytes
    pub image_height: usize,                 // 4 bytes
    pub pixel_depth: usize,                  // 4 bytes
    pub frame_count: usize,                  // 4 bytes
    pub observer: String,                    // 40 bytes
    pub instrument: String,                  // 40 bytes
    pub telescope: String,                   // 40 bytes
    pub date_time: timestamp::TimeStamp,     // 8 bytes,
    pub date_time_utc: timestamp::TimeStamp, // 8 bytes,
    pub total_size: usize,                   // Total file size (used for validation)
    file_reader: BinFileReader,
    pub source_file: String,
}

impl SerFrame {
    pub fn new(single_band_buffer: &imagebuffer::ImageBuffer, timestamp: u64) -> SerFrame {
        let mut buffer = image::Image::new_with_bands(
            single_band_buffer.width,
            single_band_buffer.height,
            1,
            single_band_buffer.mode,
        )
        .unwrap();
        buffer.add_to_each(single_band_buffer);

        SerFrame {
            buffer,
            timestamp: timestamp::TimeStamp::from_u64(timestamp),
        }
    }

    pub fn new_three_channel(
        r: &imagebuffer::ImageBuffer,
        g: &imagebuffer::ImageBuffer,
        b: &imagebuffer::ImageBuffer,
        timestamp: u64,
    ) -> SerFrame {
        let buffer = image::Image::new_from_buffers_rgb(r, g, b, r.mode).unwrap();

        SerFrame {
            buffer,
            timestamp: timestamp::TimeStamp::from_u64(timestamp),
        }
    }

    pub fn new_rgb(rgb: image::Image, timestamp: u64) -> SerFrame {
        SerFrame {
            buffer: rgb,
            timestamp: timestamp::TimeStamp::from_u64(timestamp),
        }
    }
}

impl From<SerFrame> for DataFrame {
    fn from(val: SerFrame) -> Self {
        DataFrame {
            buffer: val.buffer,
            timestamp: val.timestamp,
        }
    }
}

// Full implementation of the SER specification is sorta impractical at this time
// since I lack both the requisite test data and the motivation to actually do it.
impl SerFile {
    pub fn print_ser_header_details(&self) {
        println!("SER Header Values:");
        println!("File Id: {}", self.file_id);
        println!("Camera Series Id: {}", self.camera_series_id);
        println!("Color Id: {:?}", self.color_id);
        println!("Image Width: {}", self.image_width);
        println!("Image Height: {}", self.image_height);
        println!("Pixel Depth: {}", self.pixel_depth);
        println!("Frame Count: {}", self.frame_count);
        println!("Observer: {}", self.observer);
        println!("Instrument: {}", self.instrument);
        println!("Telescope: {}", self.telescope);
        println!("Date/Time: {:?}", self.date_time);
        println!("Date/Time UTC: {:?}", self.date_time_utc);
        println!("Total File Size: {}", self.total_size);
        println!(
            "Bytes per image: {}",
            self.image_width * self.image_height * (self.pixel_depth / 8)
        );
    }

    pub fn load_ser(file_path: &str) -> Result<SerFile> {
        let mut file_reader =
            BinFileReader::new_as_endiness(&file_path.to_string(), Endian::LittleEndian);
        let endiness = Endian::from_i32(file_reader.read_i32(22)?)?; // 4 bytes, start at 22
        file_reader.set_endiness(endiness);

        // Some values are ok to default out, others need to propogate their errors
        let ser = SerFile {
            file_id: file_reader.read_string(0, 14).unwrap_or_default(), // 14 bytes
            camera_series_id: file_reader.read_i32(14).unwrap_or(0),     // 4 bytes, start at 14
            color_id: ColorFormatId::from_i32(file_reader.read_i32(18).unwrap_or(0)), // 4 bytes, start at 18
            image_width: file_reader.read_i32(26)? as usize, // 4 bytes, start at 26
            image_height: file_reader.read_i32(30)? as usize, // 4 bytes, start at 30
            pixel_depth: file_reader.read_i32(34)? as usize, // 4 bytes, start at 34
            frame_count: file_reader.read_i32(38)? as usize, // 4 bytes, start at 38
            observer: file_reader.read_string(42, 40).unwrap_or_default(), // 40 bytes, start at 42
            instrument: file_reader.read_string(82, 40).unwrap_or_default(), // 40 bytes, start at 82
            telescope: file_reader.read_string(122, 40).unwrap_or_default(), // 40 bytes, start at 122
            date_time: timestamp::TimeStamp::from_u64(file_reader.read_u64(162)?), // 8 bytes, start at 162
            date_time_utc: timestamp::TimeStamp::from_u64(file_reader.read_u64(170)?), // 8 bytes, start at 170
            total_size: file_reader.len(),
            file_reader,
            source_file: file_path.to_string(),
        };

        if stump::is_verbose() {
            ser.print_header_details();
        }

        Ok(ser)
    }

    pub fn image_frame_size_bytes(&self) -> usize {
        self.image_width * self.image_height * (self.pixel_depth / 8)
    }

    pub fn image_frame_start_index(&self, frame_num: usize) -> usize {
        HEADER_SIZE_BYTES + (self.image_frame_size_bytes() * frame_num)
    }

    pub fn has_timestamps(&self) -> bool {
        self.total_size > self.timestamp_block_start_index()
    }

    pub fn timestamp_block_start_index(&self) -> usize {
        HEADER_SIZE_BYTES +  // Header
                (self.image_frame_size_bytes() * self.frame_count) // Frames
    }

    pub fn timestamp_start_index(&self, frame_num: usize) -> usize {
        let block_start = self.timestamp_block_start_index();
        block_start + (frame_num * TIMESTAMP_SIZE_BYTES)
    }

    pub fn expected_size(&self) -> usize {
        let has_ts = if self.has_timestamps() { 1 } else { 0 };

        HEADER_SIZE_BYTES +  // Header
            (self.image_frame_size_bytes() * self.frame_count) +   // Frames
            (8 * self.frame_count * has_ts) // Timestamps
    }

    pub fn validate_ser(&self) -> Result<()> {
        let expected_size = self.expected_size();
        if self.total_size == expected_size {
            Ok(())
        } else {
            Err(Error::msg(format!(
                "Size mismatch: {} != {}",
                self.total_size, expected_size
            )))
        }
    }

    pub fn get_ser_frame_timestamp(&self, frame_num: usize) -> Result<u64> {
        if frame_num >= self.frame_count {
            return Err(Error::msg("Frame number out of range"));
        }

        if !self.has_timestamps() {
            return Ok(0);
        }

        let timestamp_start_index = self.timestamp_start_index(frame_num);
        self.file_reader
            .read_u64_with_endiness(timestamp_start_index, Endian::NativeEndian)
    }

    pub fn get_ser_frame(&self, frame_num: usize) -> Result<SerFrame> {
        if frame_num >= self.frame_count {
            return Err(Error::msg("Frame number out of range"));
        }

        let image_frame_size_bytes = self.image_frame_size_bytes();
        let image_frame_start_index = self.image_frame_start_index(frame_num);

        info!(
            "Extracting image frame #{} of {} from {}. Size {} at byte index {}",
            frame_num,
            self.frame_count,
            self.source_file,
            image_frame_size_bytes,
            image_frame_start_index
        );

        let mut values: Vec<f32> = Vec::with_capacity(self.image_width * self.image_height);
        // values.resize(self.image_width * self.image_height, 0.0);

        let bytes_per_pixel = self.pixel_depth / 8;

        for y in 0..self.image_height {
            for x in 0..self.image_width {
                let pixel_start =
                    (x + (y * self.image_width)) * bytes_per_pixel + image_frame_start_index;

                values.push(if self.pixel_depth == 8 {
                    self.file_reader.read_u8(pixel_start)? as f32
                } else if self.pixel_depth == 16 {
                    self.file_reader.read_u16(pixel_start)? as f32
                } else {
                    panic!("Encountered unsupported pixel depth: {}", self.pixel_depth);
                });
            }
        }

        let frame_buffer = imagebuffer::ImageBuffer::from_vec_as_mode(
            &values,
            self.image_width,
            self.image_height,
            match self.pixel_depth {
                8 => ImageMode::U8BIT,
                _ => ImageMode::U16BIT,
            },
        )
        .expect("Failed to allocate image buffer");

        match self.color_id {
            ColorFormatId::Mono => Ok(SerFrame::new(
                &frame_buffer,
                self.get_ser_frame_timestamp(frame_num)
                    .expect("Failed to extract frame timestamp"),
            )),
            ColorFormatId::BayerRggb => {
                let debayered =
                    debayer::debayer(&frame_buffer, debayer::DebayerMethod::Malvar).unwrap();
                Ok(SerFrame::new_rgb(
                    debayered,
                    self.get_ser_frame_timestamp(frame_num)
                        .expect("Failed to extract frame timestamp"),
                ))
            }
            _ => {
                panic!("Unsupported color mode: {:?}", self.color_id);
            }
        }
    }
}

impl DataSource for SerFile {
    fn color_id(&self) -> ColorFormatId {
        self.color_id
    }

    fn file_id(&self) -> String {
        self.file_id.clone()
    }

    fn image_width(&self) -> usize {
        self.image_width
    }

    fn image_height(&self) -> usize {
        self.image_height
    }

    fn pixel_depth(&self) -> usize {
        self.pixel_depth
    }

    fn frame_count(&self) -> usize {
        self.frame_count
    }

    fn observer(&self) -> String {
        self.observer.clone()
    }

    fn instrument(&self) -> String {
        self.instrument.clone()
    }

    fn telescope(&self) -> String {
        self.telescope.clone()
    }

    fn date_time(&self) -> TimeStamp {
        self.date_time
    }

    fn date_time_utc(&self) -> TimeStamp {
        self.date_time_utc
    }

    fn total_file_size(&self) -> usize {
        self.total_size
    }

    fn get_frame(&self, frame_num: usize) -> Result<DataFrame> {
        Ok(self.get_ser_frame(frame_num)?.into())
    }

    fn get_frame_timestamp(&self, frame_num: usize) -> Result<TimeStamp> {
        Ok(TimeStamp::from(self.get_ser_frame_timestamp(frame_num)?))
    }

    fn source_file(&self) -> String {
        self.source_file.clone()
    }

    fn open(path: &[String]) -> Result<Self> {
        if path.len() != 1 {
            Err(Error::msg("Only one ser file supported at this time"))
        } else {
            SerFile::load_ser(&path[0])
        }
    }

    fn validate(&self) -> Result<()> {
        self.validate_ser()
    }

    fn print_header_details(&self) {
        self.print_ser_header_details();
    }

    fn file_hash(&self) -> String {
        self.source_file.clone()
    }
}
