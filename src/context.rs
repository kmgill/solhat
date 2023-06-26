use crate::calibrationframe::CalibrationImage;
use crate::drizzle::Scale;
use crate::fpmap::FpMap;
use crate::stats::ProcessStats;
use crate::target::Target;
use anyhow::Result;

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
}

pub struct ProcessContext {
    pub parameters: ProcessParameters,
    pub fp_map: FpMap,
    pub master_flat: CalibrationImage,
    pub master_dark: CalibrationImage,
    pub master_darkflat: CalibrationImage,
    pub master_bias: CalibrationImage,
    pub stats: ProcessStats,
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
            master_flat: master_flat,
            master_dark: master_darkflat,
            master_darkflat: master_dark,
            master_bias: master_bias,
            stats: ProcessStats::default(),
        };

        params.input_files.iter().for_each(|input_file| {
            pc.fp_map
                .open(input_file)
                .expect("Failed to open input file");
        });
        Ok(pc)
    }
}
