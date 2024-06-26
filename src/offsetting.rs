use anyhow::Result;
use rayon::prelude::*;

use crate::context::ProcessContext;
use crate::datasource::DataSource;
use crate::framerecord::FrameRecord;

pub fn frame_offset_analysis<C, F>(
    context: &ProcessContext<F>,
    on_frame_checked: C,
) -> Result<Vec<FrameRecord>>
where
    C: Fn(&FrameRecord) + Send + Sync + 'static,
    F: DataSource + Send + Sync + 'static,
{
    let frame_records: Vec<FrameRecord> = context
        .frame_records
        .par_iter()
        .map(|fr| {
            let mut fr_copy = fr.clone();
            let frame = fr.get_frame(context).expect("");

            fr_copy.offset = frame
                .buffer
                .calc_center_of_mass_offset(context.parameters.obj_detection_threshold as f32, 0);

            fr_copy.offset.h += context.parameters.horiz_offset as f32;
            fr_copy.offset.v += context.parameters.vert_offset as f32;

            on_frame_checked(&fr_copy);
            fr_copy
        })
        .collect();
    Ok(frame_records)
}
