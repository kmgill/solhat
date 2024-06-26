use anyhow::Result;

use crate::context::ProcessContext;
use crate::datasource::DataSource;
use crate::framerecord::FrameRecord;

pub fn frame_limit_determinate<C, F>(
    context: &ProcessContext<F>,
    on_frame_checked: C,
) -> Result<Vec<FrameRecord>>
where
    C: Fn(&FrameRecord) + Send + Sync + 'static,
    F: DataSource,
{
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

            on_frame_checked(fr);

            mn_s && mx_s
        })
        .map(|fr| fr.to_owned())
        .collect();

    frame_records.sort();
    frame_records.reverse();

    let frame_records = if let Some(limit_top_pct) = &context.parameters.top_percentage {
        let max_frame =
            ((*limit_top_pct as f32 / 100.0) * frame_records.len() as f32).round() as usize;
        frame_records[0..max_frame].to_vec()
    } else {
        frame_records
    };

    // So, if the user asked for a maximum number of frames to be used and that number does not
    // exceed the number of frames actually available, we do the limitation via a vector slice.
    // Now, as implemented, this is logically incorrect if the user passed in two or more
    // ser files as inputs, but what's some technical debt between friends, amiright?
    let frame_records = if let Some(max_frames) = &context.parameters.max_frames {
        if *max_frames <= frame_records.len() {
            frame_records[0..*max_frames].to_vec()
        } else {
            frame_records
        }
    } else {
        frame_records
    };

    Ok(frame_records)
}
