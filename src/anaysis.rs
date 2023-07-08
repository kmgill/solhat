use crate::context::ProcessContext;
use crate::framerecord::FrameRecord;
use anyhow::Result;
use rayon::prelude::*;
use sciimg::quality;

pub fn frame_sigma_analysis<F>(
    context: &ProcessContext,
    on_frame_checked: F,
) -> Result<Vec<FrameRecord>>
where
    F: Fn(&FrameRecord) + Send + Sync + 'static,
{
    frame_sigma_analysis_window_size(context, 128, on_frame_checked)
}

pub fn frame_sigma_analysis_window_size<F>(
    context: &ProcessContext,
    window_size: usize,
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
            let frame = fr.get_frame(context).expect("");

            let x = frame.buffer.width / 2 + fr_copy.offset.h as usize;
            let y = frame.buffer.height / 2 + fr_copy.offset.v as usize;

            fr_copy.sigma =
                quality::get_point_quality_estimation(&frame.buffer, window_size, x, y) as f64;

            on_frame_checked(&fr_copy);
            fr_copy
        })
        .collect();
    Ok(frame_records)
}
