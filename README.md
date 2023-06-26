# SolHat: Solar Hydrogen Alpha Telescope Imaging Pipeline

## Overview
A set of utilities and a pipeline for processing raw hydrogen-alpha solar imaging using lucky imaging and drizzle ("image stacking"). Designed as a specific use-case replacement for PIPP & Autostakkert. 

The current and planned steps include:
 * Flat and dark correction
 * Glitch frame detection
 * Quality estimation filtering 
 * Center-of-mass centering alignment
 * Cropping
 * Parallactic angle of rotation for alt-az mounting
 * Debayering (partially implemented)
 * Stacking with Drizzle (1.0x, 1.5x, 2.0x, 3.0x)
 * Support for Solar and Lunar targeting

Future Plans:
 * GUI Support
 * Planetary
 * Hot pixel detection and correction


## Contributing
Feedback, issues, and contributions are always welcomed. Should enough interest arise in contributing development efforts, I will write up a contribution guide. 


## Building from source
A working Rust (https://www.rust-lang.org/) installation is required for building.

### Clone from git
```
git clone git@github.com:kmgill/solhat.git
```

### To install, run:
```
cargo install --path .
```

## Data Format
SolHAT is designed to use `ser` files as light inputs, along with `ser`, `png` or `tif` images for calibration frames. If `ser` is used for calibration frames, the median of those inputs will be calculated automatically. 

An observation consists of five, optionally more, imaging sets:
 * Light (Chromosphere)
 * Light (Prominence, optional)
 * Light (White light, optional)
 * Flats
 * Flat darks
 * Darks
 * Bias

Each set is stored under an observation root directory in the following structure (These can be modified manually in the script):

```
    Observation Root
                     - /Sun
                     - /Sun_-_Prominence
                     - /Sun_-_Flat
                     - /Sun_-_Flat_Dark
                     - /Sun_-_Dark
                     - /Sun_-_Prominence_-_Dark
                     - /Sun_-_Bias
```
I use FireCapture, with these set as profiles. Under each profile directory is a timestamp directory, then the `ser` files. 


## Running via Automation Scripts
The automation scripts, `solhat_sun.sh`, `solhat_moon.sh`, etc, are a set of scripts which automate the usage of SolHat around a standard format imaging session (see above). They contain a number of editable parameters that can be tuned to your specific requirements or preferences. 

### Location File
The file `location.sh` needs to exist in the `$CWD` from which you run the scripts. This file is read in automatically and passed to SolHAT to correctly calculate the parallactic angle of rotatation for each frame. The format of the file is simple:
```
LOC_LATITUDE=34.12345
LOC_LONGITUDE=-118.12345
```

### Running

```
$ path/to/solhat_sun.sh /data/myobservation v1
```
where the first argument is the path to the base directory of your imaging. The second argument is a free-text value that is used as an output file suffix. 

The script will output the various master calibration images, a threshold test image, and the final image. If rerun with the same free-text argument, the script will reuse the master calibration files. 

### Hot Pixel Map
SolHat can be provided a TOML-formatted file containing information needed to replace hot pixels. This file contains the sensor width and height, and a list of x/y coordinates of pixels. By default, the scripts will look for the file at `~/.solhat/hotpixels.toml`.

An example Hot Pixel Map file:
```
sensor_width = 1936
sensor_height = 1216
hotpixels = [
        [ 1169 , 48 ],
        [ 170 , 997 ],
        [ 395 , 733 ],
        [ 1193 , 854 ],
]
```

## References:
Telea, Alexandru. (2004). An Image Inpainting Technique Based on the Fast Marching Method. Journal of Graphics Tools. 9. 10.1080/10867651.2004.10487596. 
https://www.researchgate.net/publication/238183352_An_Image_Inpainting_Technique_Based_on_the_Fast_Marching_Method

Malvar, Henrique & He, Li-wei & Cutler, Ross. (2004). High-quality linear interpolation for demosaicing of Bayer-patterned color images. Acoustics, Speech, and Signal Processing, 1988. ICASSP-88., 1988 International Conference on. 3. iii - 485. 10.1109/ICASSP.2004.1326587. 
https://www.researchgate.net/publication/4087683_High-quality_linear_interpolation_for_demosaicing_of_Bayer-patterned_color_images

Getreuer, Pascal. (2011). Malvar-He-Cutler Linear Image Demosaicking. Image Processing On Line. 1. 10.5201/ipol.2011.g_mhcd. 
https://www.researchgate.net/publication/270045976_Malvar-He-Cutler_Linear_Image_Demosaicking

Di, K., and Li, R. (2004), CAHVOR camera model and its photogrammetric conversion for planetary applications, J. Geophys. Res., 109, E04004, doi:10.1029/2003JE002199.
https://doi.org/10.1029/2003JE002199

Gennery, D.B. Generalized Camera Calibration Including Fish-Eye Lenses. Int J Comput Vision 68, 239–266 (2006). https://doi.org/10.1007/s11263-006-5168-1

Tatum, Jeremy. (2022), Stellar Atmospheres, https://phys.libretexts.org/Bookshelves/Astronomy__Cosmology/Stellar_Atmospheres_(Tatum)/06%3A_Limb_Darkening/6.01%3A_Introduction._The_Empirical_Limb-darkening