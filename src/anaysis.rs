use crate::context::ProcessContext;
use crate::framerecord::FrameRecord;
use anyhow::Result;
use rayon::prelude::*;
use sciimg::quality;

pub fn frame_sigma_analysis<F: Fn(&FrameRecord)>(
    context: &ProcessContext,
    _on_frame_checked: F,
) -> Result<Vec<FrameRecord>> {
    let frame_records: Vec<FrameRecord> = context
        .frame_records
        .par_iter()
        .map(|fr| {
            let mut fr_copy = fr.clone();
            let frame = fr.get_frame(context).expect("");
            fr_copy.sigma = quality::get_quality_estimation(&frame.buffer) as f64;
            // on_frame_checked(&fr_copy);
            fr_copy
        })
        .collect();
    Ok(frame_records)
}
