use std::cmp::Ordering;

use anyhow::Result;
use sciimg::imagebuffer::Offset;

use crate::context::ProcessContext;
use crate::datasource::{DataFrame, DataSource};
use crate::hotpixel;
use crate::target::TargetPosition;
use crate::timestamp::TimeStamp;

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
    pub fn get_frame<F: DataSource>(&self, context: &ProcessContext<F>) -> Result<DataFrame> {
        let (_, ser) = context
            .fp_map
            .get_map()
            .iter()
            .find(|(id, _)| **id == self.source_file_id)
            .unwrap();
        ser.get_frame(self.frame_id)
    }

    pub fn num_bands<F: DataSource>(&self, context: &ProcessContext<F>) -> Result<usize> {
        let f = self.get_frame(context)?;
        Ok(f.buffer.num_bands())
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
