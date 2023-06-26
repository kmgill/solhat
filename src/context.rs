use crate::calibrationframe::CalibrationImage;
use crate::drizzle::Scale;
use crate::fpmap::FpMap;
use crate::framerecord::FrameRecord;
use crate::hotpixel;
use crate::ser::SerFile;
use crate::stats::ProcessStats;
use crate::target::Target;
use anyhow::Result;
use rayon::prelude::*;
use sciimg::imagebuffer::Offset;
use sciimg::prelude::ImageBuffer;

#[derive(Debug, Clone)]
pub struct ProcessParameters {
    pub input_files: Vec<String>,
    pub obj_detection_threshold: f64,
    pub obs_latitude: f64,
    pub obs_longitude: f64,
    pub target: Target,
    pub crop_width: Option<usize>,
    pub crop_height: Option<usize>,
    pub max_frames: Option<usize>,
    pub min_sigma: Option<f64>,
    pub max_sigma: Option<f64>,
    pub top_percentage: Option<f64>,
    pub drizzle_scale: Scale,
    pub initial_rotation: f64,
    pub flat_inputs: Option<String>,
    pub dark_inputs: Option<String>,
    pub darkflat_inputs: Option<String>,
    pub bias_inputs: Option<String>,
    pub hot_pixel_map: Option<String>,
}

pub struct ProcessContext {
    pub parameters: ProcessParameters,
    pub fp_map: FpMap,
    pub master_flat: CalibrationImage,
    pub master_dark: CalibrationImage,
    pub master_darkflat: CalibrationImage,
    pub master_bias: CalibrationImage,
    pub stats: ProcessStats,
    pub frame_records: Vec<FrameRecord>,
    pub hotpixel_mask: Option<ImageBuffer>,
}

fn load_frame_records_for_ser(ser_file: &SerFile, number_of_frames: usize) -> Vec<FrameRecord> {
    let frame_count = if ser_file.frame_count > number_of_frames {
        number_of_frames
    } else {
        ser_file.frame_count
    };

    (0..frame_count)
        .into_iter()
        .map(|i| FrameRecord {
            source_file_id: ser_file.source_file.to_string(),
            frame_id: i,
            frame_width: ser_file.image_width,
            frame_height: ser_file.image_height,
            sigma: 0.0,
            computed_rotation: 0.0,
            offset: Offset { h: 0.0, v: 0.0 },
        })
        .collect::<Vec<FrameRecord>>()
}

fn load_hot_pixel_mask(file_path: &Option<String>) -> Result<Option<ImageBuffer>> {
    if let Some(hpm_path) = file_path {
        Ok(Some(hotpixel::create_hotpixel_mask(
            &hotpixel::load_hotpixel_map(hpm_path)?,
        )?))
    } else {
        Ok(None)
    }
}

impl ProcessContext {
    pub fn create_with_calibration_frames(
        params: &ProcessParameters,
        master_flat: CalibrationImage,
        master_darkflat: CalibrationImage,
        master_dark: CalibrationImage,
        master_bias: CalibrationImage,
    ) -> Result<Self> {
        let mut pc = ProcessContext {
            parameters: params.to_owned(),
            fp_map: FpMap::new(),
            master_flat,
            master_dark,
            master_darkflat,
            master_bias,
            stats: ProcessStats::default(),
            frame_records: vec![],
            hotpixel_mask: load_hot_pixel_mask(&params.hot_pixel_map)?,
        };

        params.input_files.iter().for_each(|input_file| {
            info!("Loading input file: {}", input_file);
            pc.fp_map
                .open(input_file)
                .expect("Failed to open input file");
        });

        pc.frame_records = pc
            .fp_map
            .get_map()
            .par_iter()
            .map(|(_, ser)| load_frame_records_for_ser(ser, params.max_frames.unwrap_or(100000000)))
            .collect::<Vec<Vec<FrameRecord>>>()
            .iter()
            .flatten()
            .map(|fr| fr.to_owned())
            .collect::<Vec<FrameRecord>>();

        Ok(pc)
    }
}
