use sciimg::image::Image;
use sciimg::imagebuffer::Offset;
use sciimg::imagerot;

pub trait CenterOfMass {
    fn calc_center_of_mass_offset_with_rotation(
        &self,
        threshold: f32,
        rotation: f32,
        band: usize,
    ) -> Offset;
}

impl CenterOfMass for Image {
    fn calc_center_of_mass_offset_with_rotation(
        &self,
        threshold: f32,
        rotation: f32,
        band: usize,
    ) -> Offset {
        let rotated = imagerot::rotate(self.get_band(band), rotation).unwrap();
        rotated.calc_center_of_mass_offset(threshold)
    }
}
