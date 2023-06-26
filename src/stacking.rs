use crate::framerecord::FrameRecord;
use crate::{context::ProcessContext, drizzle::BilinearDrizzle};
use anyhow::{anyhow, Result};
use rayon::prelude::*;

pub fn process_frame_stacking<F>(
    context: &ProcessContext,
    on_frame_checked: F,
) -> Result<BilinearDrizzle>
where
    F: Fn(&FrameRecord) + Send + Sync + 'static,
{
    if context.frame_records.is_empty() {
        return Err(anyhow!("No frames to stack!"));
    }

    let mut master_drizzle = BilinearDrizzle::new(
        context.frame_records[0].frame_width,
        context.frame_records[0].frame_height,
        context.parameters.drizzle_scale,
        3,
    );

    let num_per_chunk = context.frame_records.len() / num_cpus::get();

    let sub_drizzles: Vec<BilinearDrizzle> = context
        .frame_records
        .par_chunks(num_per_chunk)
        .map(|record_chunk| {
            let mut drizzle = BilinearDrizzle::new(
                context.frame_records[0].frame_width,
                context.frame_records[0].frame_height,
                context.parameters.drizzle_scale,
                3,
            );

            record_chunk.iter().for_each(|fr| {
                let frame = fr
                    .get_calibrated_frame(context)
                    .expect("Failed to retrieve calibrated frame");

                drizzle
                    .add_with_transform(&frame.buffer, &fr.offset, fr.computed_rotation)
                    .expect("Failed to drizzle frame onto buffer");

                on_frame_checked(fr);
            });

            drizzle
        })
        .collect();

    // Combines all the sub drizzle buffers into the master drizzle
    sub_drizzles.iter().for_each(|d| {
        master_drizzle.add_drizzle(d).unwrap();
    });

    Ok(master_drizzle)
}
