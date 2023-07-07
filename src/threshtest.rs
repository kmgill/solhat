use sciimg::prelude::*;
use sciimg::Dn;
use itertools::iproduct;

pub fn compute_rgb_threshtest_image(frame: &Image, threshold: Dn) -> Image {

    let mut out = Image::new_with_bands(frame.width, frame.height, 3, frame.get_mode()).unwrap();

    let max_val = if frame.get_mode() == ImageMode::U8BIT { 255.0 } else { 65535.0 };

    iproduct!(0..frame.height, 0..frame.width).for_each(|(y, x)|{

        let (r, g, b) = if frame.num_bands() == 1 {
            let v = frame.get_band(0).get(x,y);
            let t = if v >= threshold {
                max_val
            } else {
                v
            };
            (t, v, v)
        } else  {
            let r = frame.get_band(0).get(x, y);
            let g = frame.get_band(1).get(x, y);
            let b = frame.get_band(2).get(x, y);
            (
                if r >= threshold { max_val} else { r },
                if g >= threshold { max_val} else { g },
                if b >= threshold { max_val} else { b }
            )
        };

        out.put(x, y, r, 0);
        out.put(x, y, g, 1);
        out.put(x, y, b, 2);
    });

    out
}

pub fn compute_threshtest_image(frame: &Image, threshold: Dn) -> ImageBuffer {
    info!(
        "Creating test visualization buffer of size {}x{}",
        frame.width, frame.height
    );
    let mut out_img =
        ImageBuffer::new_with_fill_as_mode(frame.width, frame.height, 0.0, ImageMode::U8BIT)
            .unwrap();

    info!("Checking threshold value {} across first frame", threshold);
    for y in 0..frame.height {
        for x in 0..frame.width {
            let mut v = 0.0;
            for b in 0..frame.num_bands() {
                v += frame.get_band(b).get(x, y);
            }
            v /= frame.num_bands() as Dn;
            if v > threshold {
                out_img.put(x, y, 65535.0);
            } else {
                out_img.put(x, y, frame.get_band(0).get(x, y));
            }
        }
    }
    out_img.normalize_mut(0.0, 255.0);
    out_img
}
