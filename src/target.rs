use crate::{lunar, parallacticangle, solar, timestamp::TimeStamp};
use anyhow::Result;

pub struct TargetPosition {
    pub rotation: f64,
    pub altitude: f64,
    pub azimuth: f64,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Target {
    Sun,
    Moon,
    // Mercury,
    // Venus,
    // Mars,
    // Jupiter,
    // Saturn,
    // Uranus,
    // Neptune,
}

impl Target {
    pub fn from(s: &str) -> Result<Target> {
        match s.to_uppercase().as_str() {
            "MOON" => Ok(Target::Moon),
            "SUN" => Ok(Target::Sun),
            _ => Err(anyhow!("Invalid target supplied: '{}'", s)),
        }
    }

    pub fn position_from_lat_lon_and_time(
        &self,
        obs_latitude: f64,
        obs_longitude: f64,
        ts: &TimeStamp,
    ) -> Result<TargetPosition> {
        let (altitude, azimuth) = match self {
            Target::Moon => {
                info!("Calculating position for Moon");
                lunar::position_from_lat_lon_and_time(obs_latitude, obs_longitude, ts)
            }
            Target::Sun => {
                info!("Calculating position for Sun");
                solar::position_from_lat_lon_and_time(obs_latitude, obs_longitude, ts)
            }
        };

        let rotation = parallacticangle::from_lat_azimuth_altitude(obs_latitude, azimuth, altitude);

        Ok(TargetPosition {
            rotation,
            altitude,
            azimuth,
        })
    }
}
