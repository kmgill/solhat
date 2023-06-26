use crate::context::ProcessContext;
use crate::framerecord::FrameRecord;
use anyhow::Result;

pub fn frame_limit_determinate<F>(
    context: &ProcessContext,
    on_frame_checked: F,
) -> Result<Vec<FrameRecord>>
where
    F: Fn(&FrameRecord) + Send + Sync + 'static,
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

    // So, if the user asked for a maximum number of frames to be used and that number does not
    // exceed the number of frames actually available, we do the limitation via a vector slice.
    // Now, as implemented, this is logically incorrect if the user passed in two or more
    // ser files as inputs, but what's some technical debt between friends, amiright?
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
