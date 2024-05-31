use anyhow::{anyhow, Result};
use rayon::prelude::*;

use crate::context::ProcessContext;
use crate::datasource::DataSource;
use crate::drizzle::{
    AverageStackBuffer, BilinearDrizzle, MedianStackBuffer, MinimumStackBuffer, StackAlgorithm,
    StackAlgorithmImpl, StackBuffer,
};
use crate::framerecord::FrameRecord;
use sciimg::prelude::Image;

pub fn process_frame_stacking<C, F>(
    context: &ProcessContext<F>,
    on_frame_checked: C,
) -> Result<Image>
where
    C: Fn(&FrameRecord) + Send + Sync + 'static,
    F: DataSource + Send + Sync + 'static,
{
    if context.frame_records.is_empty() {
        return Err(anyhow!("No frames to stack!"));
    }

    match context.parameters.algorithm.allow_parallel() {
        true => process_frame_stacking_parallel(context, on_frame_checked),
        false => process_frame_stacking_linear(context, on_frame_checked),
    }
}

pub fn process_frame_stacking_parallel<C, F>(
    context: &ProcessContext<F>,
    on_frame_checked: C,
) -> Result<Image>
where
    C: Fn(&FrameRecord) + Send + Sync + 'static,
    F: DataSource + Send + Sync + 'static,
{
    if context.frame_records.is_empty() {
        return Err(anyhow!("No frames to stack!"));
    }

    let stack_algorithm = context.parameters.algorithm;

    let in_width = context
        .parameters
        .crop_width
        .unwrap_or(context.frame_records[0].frame_width);
    let in_height = context
        .parameters
        .crop_height
        .unwrap_or(context.frame_records[0].frame_height);

    let out_width = (in_width as f32 * context.parameters.drizzle_scale.value()).ceil() as usize;
    let out_height = (in_height as f32 * context.parameters.drizzle_scale.value()).ceil() as usize;

    let num_source_bands = context.frame_records[0].num_bands(&context)?;

    let mut master_drizzle: BilinearDrizzle = BilinearDrizzle::new(
        context
            .parameters
            .crop_width
            .unwrap_or(context.frame_records[0].frame_width),
        context
            .parameters
            .crop_height
            .unwrap_or(context.frame_records[0].frame_height),
        context.parameters.drizzle_scale,
        context.parameters.horiz_offset,
        context.parameters.vert_offset,
        match stack_algorithm {
            StackAlgorithm::Average => StackAlgorithmImpl::Average(AverageStackBuffer::new(
                out_width,
                out_height,
                num_source_bands,
            )),
            StackAlgorithm::Median => StackAlgorithmImpl::Median(MedianStackBuffer::new(
                out_width,
                out_height,
                num_source_bands,
            )),
            StackAlgorithm::Minimum => StackAlgorithmImpl::Minimum(MinimumStackBuffer::new(
                out_width,
                out_height,
                num_source_bands,
            )),
        },
    );

    let num_per_chunk = context.frame_records.len() / num_cpus::get();

    let sub_drizzles: Vec<BilinearDrizzle> = context
        .frame_records
        .par_chunks(num_per_chunk)
        .map(|record_chunk| {
            let mut drizzle =
                BilinearDrizzle::new(
                    in_width,
                    in_height,
                    context.parameters.drizzle_scale,
                    context.parameters.horiz_offset,
                    context.parameters.vert_offset,
                    match stack_algorithm {
                        StackAlgorithm::Average => StackAlgorithmImpl::Average(
                            AverageStackBuffer::new(out_width, out_height, num_source_bands),
                        ),
                        StackAlgorithm::Median => StackAlgorithmImpl::Median(
                            MedianStackBuffer::new(out_width, out_height, num_source_bands),
                        ),
                        StackAlgorithm::Minimum => StackAlgorithmImpl::Minimum(
                            MinimumStackBuffer::new(out_width, out_height, num_source_bands),
                        ),
                    },
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

    // // Combines all the sub drizzle buffers into the master drizzle
    sub_drizzles.iter().for_each(|d| {
        master_drizzle.add_drizzle(d).unwrap();
    });

    master_drizzle.get_finalized()
}

pub fn process_frame_stacking_linear<C, F>(
    context: &ProcessContext<F>,
    on_frame_checked: C,
) -> Result<Image>
where
    C: Fn(&FrameRecord) + Send + Sync + 'static,
    F: DataSource + Send + Sync + 'static,
{
    if context.frame_records.is_empty() {
        return Err(anyhow!("No frames to stack!"));
    }

    let stack_algorithm = context.parameters.algorithm;

    let in_width = context
        .parameters
        .crop_width
        .unwrap_or(context.frame_records[0].frame_width);
    let in_height = context
        .parameters
        .crop_height
        .unwrap_or(context.frame_records[0].frame_height);

    let out_width = (in_width as f32 * context.parameters.drizzle_scale.value()).ceil() as usize;
    let out_height = (in_height as f32 * context.parameters.drizzle_scale.value()).ceil() as usize;

    let num_source_bands = context.frame_records[0].num_bands(&context)?;

    let mut master_drizzle: BilinearDrizzle = BilinearDrizzle::new(
        context
            .parameters
            .crop_width
            .unwrap_or(context.frame_records[0].frame_width),
        context
            .parameters
            .crop_height
            .unwrap_or(context.frame_records[0].frame_height),
        context.parameters.drizzle_scale,
        context.parameters.horiz_offset,
        context.parameters.vert_offset,
        match stack_algorithm {
            StackAlgorithm::Average => StackAlgorithmImpl::Average(AverageStackBuffer::new(
                out_width,
                out_height,
                num_source_bands,
            )),
            StackAlgorithm::Median => StackAlgorithmImpl::Median(MedianStackBuffer::new(
                out_width,
                out_height,
                num_source_bands,
            )),
            StackAlgorithm::Minimum => StackAlgorithmImpl::Minimum(MinimumStackBuffer::new(
                out_width,
                out_height,
                num_source_bands,
            )),
        },
    );

    context.frame_records.iter().for_each(|fr| {
        let frame = fr
            .get_calibrated_frame(context)
            .expect("Failed to retrieve calibrated frame");

        master_drizzle
            .add_with_transform(&frame.buffer, &fr.offset, fr.computed_rotation)
            .expect("Failed to drizzle frame onto buffer");

        on_frame_checked(fr);
    });

    master_drizzle.get_finalized()
}
