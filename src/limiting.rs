use crate::context::ProcessContext;
use crate::framerecord::FrameRecord;
use anyhow::Result;

pub fn frame_limit_determinate<F: Fn(&FrameRecord)>(
    context: &ProcessContext,
    _on_frame_checked: F,
) -> Result<Vec<FrameRecord>> {
    let mut frame_records: Vec<FrameRecord> = context
        .frame_records
        .iter()
        .filter(|fr| {
            let mn_s = if let Some(minsigma) = &context.parameters.min_sigma {
                fr.sigma >= *minsigma
            } else {
                true
            };
            let mx_s = if let Some(maxsigma) = &context.parameters.max_sigma {
                fr.sigma <= *maxsigma
            } else {
                true
            };

            mn_s && mx_s
        })
        .map(|fr| fr.to_owned())
        .collect();

    let frame_records = if let Some(max_frames) = &context.parameters.max_frames {
        if *max_frames <= frame_records.len() {
            frame_records.sort();
            frame_records.reverse();
            frame_records[0..*max_frames].to_vec()
        } else {
            frame_records
        }
    } else {
        frame_records
    };

    Ok(frame_records)
}
