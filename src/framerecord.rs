use crate::context::ProcessContext;
use crate::ser::SerFrame;
use crate::target::Target;
use crate::target::TargetPosition;
use crate::timestamp::TimeStamp;
use anyhow::Result;

pub struct FrameRecord {
    pub source_file_id: String,
    pub frame_id: usize,
    pub sigma: f64,
}

impl FrameRecord {
    pub fn get_frame(&self, context: &ProcessContext) -> Result<SerFrame> {
        let (_, ser) = context
            .fp_map
            .get_map()
            .iter()
            .find(|(id, _)| **id == self.source_file_id)
            .unwrap();
        ser.get_frame(self.frame_id)
    }

    pub fn get_calibrated_frame(&self, context: &ProcessContext) -> Result<SerFrame> {
        let mut frame_buffer = self.get_frame(context)?;

        frame_buffer.buffer.calibrate2(
            &context.master_flat.image,
            &context.master_dark.image,
            &context.master_darkflat.image,
            &context.master_bias.image,
        );
        Ok(frame_buffer)
    }

    pub fn get_timestamp(&self, context: &ProcessContext) -> Result<TimeStamp> {
        let frame_buffer = self.get_frame(context)?;
        Ok(frame_buffer.timestamp)
    }

    pub fn get_rotation_for_time(
        &self,
        target: Target,
        obs_latitude: f64,
        obs_longitude: f64,
        context: &ProcessContext,
    ) -> Result<TargetPosition> {
        let ts = self.get_timestamp(context)?;
        target.position_from_lat_lon_and_time(obs_latitude, obs_longitude, &ts)
    }
}
