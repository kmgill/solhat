use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use sciimg::imagebuffer::Offset;

use crate::context::ProcessContext;
use crate::datasource::{DataFrame, DataSource};
use crate::hotpixel;
use crate::target::TargetPosition;
use crate::timestamp::TimeStamp;

lazy_static! {
    static ref FRAME_CACHE: Arc<Mutex<HashMap<String, DataFrame>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

#[allow(dead_code)]
fn make_hash_id(file: &str, id: usize) -> String {
    format!("{}-{}", file, id)
}

#[allow(dead_code)]
fn cache_has_frame(file: &str, id: usize) -> bool {
    let hash_id = make_hash_id(file, id);
    FRAME_CACHE.lock().unwrap().contains_key(&hash_id)
}

#[allow(dead_code)]
fn put_frame(file: &str, id: usize, frame: &DataFrame) {
    let hash_id = make_hash_id(file, id);
    FRAME_CACHE
        .lock()
        .unwrap()
        .insert(hash_id, frame.to_owned());
}

#[allow(dead_code)]
fn get_frame(file: &str, id: usize) -> Option<DataFrame> {
    let hash_id = make_hash_id(file, id);
    if cache_has_frame(file, id) {
        Some(
            FRAME_CACHE
                .lock()
                .unwrap()
                .get(&hash_id)
                .unwrap()
                .to_owned(),
        )
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct FrameRecord {
    pub source_file_id: String, // The input filename of the ser file
    pub frame_id: usize,        // The index of the frame within the ser file
    pub frame_width: usize,     // The width, in pixels, of the frame
    pub frame_height: usize,    // The height, in pixels, of the frame
    pub sigma: f64,             // The computed quality (sigma) value of the raw image
    pub computed_rotation: f64, // The parallactic angle of rotation, in radians
    pub offset: Offset,         // The center-of-mass offset needed to center the target
}

impl FrameRecord {
    pub fn get_frame<F:DataSource>(&self, context: &ProcessContext<F>) -> Result<DataFrame> {
        let (_, ser) = context
            .fp_map
            .get_map()
            .iter()
            .find(|(id, _)| **id == self.source_file_id)
            .unwrap();
        ser.get_frame(self.frame_id)
        // if cache_has_frame(&self.source_file_id, self.frame_id) {
        //     Ok(get_frame(&self.source_file_id, self.frame_id).unwrap())
        // } else {
        //     let (_, ser) = context
        //         .fp_map
        //         .get_map()
        //         .iter()
        //         .find(|(id, _)| **id == self.source_file_id)
        //         .unwrap();
        //     let frame = ser.get_frame(self.frame_id)?;
        //     put_frame(&self.source_file_id, self.frame_id, &frame);
        //     Ok(frame)
        // }
    }

    pub fn get_calibrated_frame<F: DataSource>(
        &self,
        context: &ProcessContext<F>,
    ) -> Result<DataFrame> {
        let mut frame_buffer = self.get_frame(context)?;

        frame_buffer.buffer.calibrate2(
            &context.master_flat.image,
            &context.master_dark.image,
            &context.master_darkflat.image,
            &context.master_bias.image,
        );

        if let Some(hpm) = &context.hotpixel_mask {
            frame_buffer.buffer = hotpixel::replace_hot_pixels(&mut frame_buffer.buffer, hpm);
        }

        Ok(frame_buffer)
    }

    pub fn get_timestamp<F: DataSource>(&self, context: &ProcessContext<F>) -> Result<TimeStamp> {
        let (_, ser) = context
            .fp_map
            .get_map()
            .iter()
            .find(|(id, _)| **id == self.source_file_id)
            .unwrap();
        ser.get_frame_timestamp(self.frame_id)
    }

    pub fn get_rotation_for_time<F: DataSource>(
        &self,
        context: &ProcessContext<F>,
    ) -> Result<TargetPosition> {
        let ts = self.get_timestamp(context)?;
        context.parameters.target.position_from_lat_lon_and_time(
            context.parameters.obs_latitude,
            context.parameters.obs_longitude,
            &ts,
        )
    }
}

impl Ord for FrameRecord {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.sigma < other.sigma {
            Ordering::Less
        } else if self.sigma == other.sigma {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    }
}

impl PartialOrd for FrameRecord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for FrameRecord {
    fn eq(&self, other: &Self) -> bool {
        self.sigma == other.sigma
    }
}

impl Eq for FrameRecord {}
