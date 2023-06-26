use serde::Serialize;

#[derive(Debug, Default, Clone, Serialize)]
pub struct ProcessStats {
    pub total_frames: usize,
    pub num_frames_used: usize,
    pub min_sigma: f32,
    pub max_sigma: f32,
    pub num_frames_discarded: usize,
    pub num_frames_discarded_min_sigma: usize,
    pub num_frames_discarded_max_sigma: usize,
    pub num_frames_discarded_top_percentage: usize,
    pub initial_rotation: f32,
    pub quality_values: Vec<f32>,
}
