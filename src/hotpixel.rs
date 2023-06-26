use anyhow::anyhow;
use anyhow::Result;
use sciimg::path;
use sciimg::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct HotPixelMap {
    pub hotpixels: Vec<Vec<usize>>,
    pub sensor_width: usize,
    pub sensor_height: usize,
}

pub fn load_hotpixel_map(file_path: &str) -> Result<HotPixelMap> {
    if !path::file_exists(file_path) {
        Err(anyhow!("File not found: {}", file_path))
    } else {
        let t = std::fs::read_to_string(file_path)?;
        Ok(toml::from_str(&t)?)
    }
}

pub fn create_hotpixel_mask(map: &HotPixelMap) -> Result<ImageBuffer> {
    let mut mask = ImageBuffer::new(map.sensor_width, map.sensor_height)?;

    map.hotpixels.iter().for_each(|xy| {
        if xy.len() != 2 {
            warn!("Invalid pixel location: {:?}", xy);
        } else {
            let x = xy[0];
            let y = xy[1];
            info!("Hot pixel: x = {}, y = {}", x, y);
            mask.put(x, y, 255.0);
        }
    });
    Ok(mask)
}

pub fn replace_hot_pixels(image: &mut Image, mask: &ImageBuffer) -> Image {
    let mut copy = image.clone();
    copy.apply_inpaint_fix(mask);
    copy
}
