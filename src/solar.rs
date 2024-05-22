use crate::timestamp;
use chrono::{Datelike, NaiveDateTime, NaiveTime, Timelike};

pub mod util {

    // Decimal Degrees = degrees + (minutes/60) + (seconds/3600)
    pub fn hms_to_dd(degrees: f64, minutes: f64, seconds: f64) -> f64 {
        degrees + (minutes / 60.0) + (seconds / 3600.0)
    }
}

pub fn position_from_lat_lon_and_time(lat: f64, lon: f64, ts: &timestamp::TimeStamp) -> (f64, f64) {
    let unixtime = ts.to_unix_timestamp();
    // let utc = ts.to_chrono_utc();
    info!("Time {:?} converted to unix timestamp {}", ts, unixtime);
    let pos = sun::pos(unixtime * 1000, lat, lon);
    (pos.altitude.to_degrees(), pos.azimuth.to_degrees())

    // let pos = solar_position(utc, lat, lon);
    // (pos.corrected_solar_elevation_angle, pos.solar_azimuth_angle)
}

// #[derive(Debug, Clone)]
// pub struct SolarPosition {
//     solar_declination: f64,
//     corrected_solar_elevation_angle: f64,
//     solar_azimuth_angle: f64,
// }

// // https://github.com/mfreeborn/heliocron/blob/master/src/calc.rs
// fn solar_position(date: DateTime<Utc>, lat: f64, lon: f64) -> SolarPosition {
//     let time_zone = Local::now().offset().local_minus_utc() as f64;
//     let julian_date = date.naive_utc().to_julian_date();
//     let julian_century = (julian_date - 2451545.0) / 36525.0;

//     let geometric_solar_mean_longitude =
//         (280.46646 + julian_century * (36000.76983 + julian_century * 0.0003032)) % 360.0;

//     let solar_mean_anomaly =
//         357.52911 + julian_century * (35999.05029 - 0.0001537 * julian_century);

//     let eccent_earth_orbit =
//         0.016708634 - julian_century * (0.000042037 + 0.0000001267 * julian_century);

//     let equation_of_the_center = solar_mean_anomaly.to_radians().sin()
//         * (1.914602 - julian_century * (0.004817 + 0.000014 * julian_century))
//         + (2.0 * solar_mean_anomaly).to_radians().sin() * (0.019993 - 0.000101 * julian_century)
//         + (3.0 * solar_mean_anomaly).to_radians().sin() * 0.000289;

//     let solar_true_longitude = geometric_solar_mean_longitude + equation_of_the_center;

//     let solar_apparent_longitude = solar_true_longitude
//         - 0.00569
//         - 0.00478 * (125.04 - 1934.136 * julian_century).to_radians().sin();

//     let mean_oblique_ecliptic = 23.0
//         + (26.0
//             + (21.448
//                 - julian_century
//                     * (46.815 + julian_century * (0.00059 - julian_century * 0.001813)))
//                 / 60.0)
//             / 60.0;

//     let oblique_corrected =
//         mean_oblique_ecliptic + 0.00256 * (125.04 - 1934.136 * julian_century).to_radians().cos();

//     let solar_declination = ((oblique_corrected.to_radians().sin()
//         * solar_apparent_longitude.to_radians().sin())
//     .asin())
//     .to_degrees();

//     let var_y = (oblique_corrected / 2.0).to_radians().tan().powi(2);

//     let equation_of_time = 4.0
//         * (var_y * (geometric_solar_mean_longitude.to_radians() * 2.0).sin()
//             - 2.0 * eccent_earth_orbit * solar_mean_anomaly.to_radians().sin()
//             + 4.0
//                 * eccent_earth_orbit
//                 * var_y
//                 * solar_mean_anomaly.to_radians().sin()
//                 * (geometric_solar_mean_longitude.to_radians() * 2.0).cos()
//             - 0.5 * var_y * var_y * (geometric_solar_mean_longitude.to_radians() * 4.0).sin()
//             - 1.25
//                 * eccent_earth_orbit
//                 * eccent_earth_orbit
//                 * (solar_mean_anomaly.to_radians() * 2.0).sin())
//         .to_degrees();

//     // let solar_noon_fraction =
//     //     (720.0 - 4.0 * *lon - equation_of_time + time_zone * 60.0) / 1440.0;

//     let true_solar_time = (date.time().day_fraction() * 1440.0 + equation_of_time + 4.0 * lon
//         - 60.0 * time_zone)
//         % 1440.0;

//     let true_hour_angle = if true_solar_time / 4.0 < 0.0 {
//         true_solar_time / 4.0 + 180.0
//     } else {
//         true_solar_time / 4.0 - 180.0
//     };

//     let solar_zenith_angle = (lat.to_radians().sin() * solar_declination.to_radians().sin()
//         + lat.to_radians().cos()
//             * solar_declination.to_radians().cos()
//             * true_hour_angle.to_radians().cos())
//     .acos()
//     .to_degrees();

//     let solar_elevation_angle = 90.0 - solar_zenith_angle;

//     let atmospheric_refraction = (if solar_elevation_angle > 85.0 {
//         0.0
//     } else if solar_elevation_angle > 5.0 {
//         58.1 / solar_elevation_angle.to_radians().tan()
//             - 0.07 / solar_elevation_angle.to_radians().tan().powi(3)
//             + 0.000086 / solar_elevation_angle.to_radians().tan().powi(5)
//     } else if solar_elevation_angle > -0.575 {
//         1735.0
//             + solar_elevation_angle
//                 * (103.4 + solar_elevation_angle * (-12.79 + solar_elevation_angle * 0.711))
//     } else {
//         -20.772 / solar_elevation_angle.to_radians().tan()
//     } / 3600.0);

//     let corrected_solar_elevation_angle = solar_elevation_angle + atmospheric_refraction;

//     let solar_azimuth_angle = if true_hour_angle > 0. {
//         (((lat.to_radians().sin() * solar_zenith_angle.to_radians().cos()
//             - solar_declination.to_radians().sin())
//             / (lat.to_radians().cos() * solar_zenith_angle.to_radians().sin()))
//         .acos()
//         .to_degrees())
//             + 180. % 360.
//     } else {
//         (540.
//             - (((lat.to_radians().sin() * solar_zenith_angle.to_radians().cos()
//                 - solar_declination.to_radians().sin())
//                 / (lat.to_radians().cos() * solar_zenith_angle.to_radians().sin()))
//             .acos()
//             .to_degrees()))
//             % 360.
//     };

//     SolarPosition {
//         solar_declination,
//         corrected_solar_elevation_angle,
//         solar_azimuth_angle,
//     }
// }

pub trait DateTimeExt {
    fn to_julian_date(&self) -> f64;
}

impl DateTimeExt for NaiveDateTime {
    fn to_julian_date(&self) -> f64 {
        let (year, month, day): (i32, i32, i32) =
            (self.year(), self.month() as i32, self.day() as i32);

        let julian_day =
            (367 * year - 7 * (year + (month + 9) / 12) / 4 + 275 * month / 9 + day + 1721014)
                as f64;

        // Adjust for the epoch starting at 12:00 UTC.
        let hour_part = if self.hour() >= 12 {
            (self.hour() - 12) as f64 / 24.0
        } else {
            (self.hour() as f64 / 24.0) - 0.5
        };

        let time_part =
            hour_part + (self.minute() as f64 / 1440.0) + (self.second() as f64 / 86400.0);

        julian_day + time_part
    }
}

pub trait NaiveTimeExt {
    fn day_fraction(&self) -> f64;
}

impl NaiveTimeExt for NaiveTime {
    fn day_fraction(&self) -> f64 {
        self.num_seconds_from_midnight() as f64 / 86400.0
    }
}
