use crate::context::ProcessContext;
use crate::framerecord::FrameRecord;
use anyhow::Result;
use rayon::prelude::*;

/// Determines the parallactic angle of rotation for each frame
pub fn frame_rotation_analysis<F>(
    context: &ProcessContext,
    on_frame_checked: F,
) -> Result<Vec<FrameRecord>>
where
    F: Fn(&FrameRecord) + Send + Sync + 'static,
{
    let frame_records: Vec<FrameRecord> = context
        .frame_records
        .par_iter()
        .map(|fr| {
            let mut fr_copy = fr.clone();
            fr_copy.computed_rotation = (context.parameters.initial_rotation
                - fr.get_rotation_for_time(context).unwrap().rotation)
                .to_radians();
            on_frame_checked(&fr_copy);
            fr_copy
        })
        .collect();
    Ok(frame_records)
}
