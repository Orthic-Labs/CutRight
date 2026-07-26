use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TimestampError {
    #[error("fps numerator and denominator must be positive")]
    InvalidFps,
    #[error("time values must be non-negative")]
    NegativeTime,
    #[error("source range must end after it starts")]
    InvalidSourceRange,
    #[error("output range must end after it starts")]
    InvalidOutputRange,
    #[error("speed must be greater than zero")]
    InvalidSpeed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimestampMs(pub i64);

impl TimestampMs {
    pub fn new(value: i64) -> Result<Self, TimestampError> {
        if value < 0 {
            return Err(TimestampError::NegativeTime);
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RationalFps {
    pub num: u32,
    pub den: u32,
}

impl RationalFps {
    pub fn new(num: u32, den: u32) -> Result<Self, TimestampError> {
        if num == 0 || den == 0 {
            return Err(TimestampError::InvalidFps);
        }
        Ok(Self { num, den })
    }

    pub fn frames_to_ms(self, frames: i64) -> Result<i64, TimestampError> {
        if frames < 0 {
            return Err(TimestampError::NegativeTime);
        }
        Ok(((frames as i128 * self.den as i128 * 1000) / self.num as i128) as i64)
    }

    pub fn ms_to_frames(self, milliseconds: i64) -> Result<i64, TimestampError> {
        if milliseconds < 0 {
            return Err(TimestampError::NegativeTime);
        }
        Ok(
            ((milliseconds as i128 * self.num as i128 + (self.den as i128 * 1000 / 2))
                / (self.den as i128 * 1000)) as i64,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimeMapping {
    pub source_start_ms: i64,
    pub source_end_ms: i64,
    pub output_start_ms: i64,
    pub output_end_ms: i64,
    pub speed: f64,
}

impl TimeMapping {
    pub fn validate(&self) -> Result<(), TimestampError> {
        if self.source_start_ms < 0
            || self.source_end_ms < 0
            || self.output_start_ms < 0
            || self.output_end_ms < 0
        {
            return Err(TimestampError::NegativeTime);
        }
        if self.source_end_ms <= self.source_start_ms {
            return Err(TimestampError::InvalidSourceRange);
        }
        if self.output_end_ms <= self.output_start_ms {
            return Err(TimestampError::InvalidOutputRange);
        }
        if !self.speed.is_finite() || self.speed <= 0.0 {
            return Err(TimestampError::InvalidSpeed);
        }
        Ok(())
    }

    pub fn source_to_output_ms(&self, source_ms: i64) -> Result<i64, TimestampError> {
        self.validate()?;
        let offset = source_ms - self.source_start_ms;
        Ok(self.output_start_ms + (offset as f64 / self.speed).round() as i64)
    }

    pub fn output_to_source_ms(&self, output_ms: i64) -> Result<i64, TimestampError> {
        self.validate()?;
        let offset = output_ms - self.output_start_ms;
        Ok(self.source_start_ms + (offset as f64 * self.speed).round() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_fps_round_trip_is_bounded_to_one_frame() {
        let fps = RationalFps::new(30_000, 1_001).unwrap();
        for frame in [0, 1, 24, 300, 1_000, 10_000] {
            let ms = fps.frames_to_ms(frame).unwrap();
            let round_trip = fps.ms_to_frames(ms).unwrap();
            assert!(
                (round_trip - frame).abs() <= 1,
                "frame={frame} round_trip={round_trip}"
            );
        }
    }

    #[test]
    fn time_mapping_round_trips_source_time() {
        let mapping = TimeMapping {
            source_start_ms: 812,
            source_end_ms: 6_730,
            output_start_ms: 0,
            output_end_ms: 5_918,
            speed: 1.0,
        };
        for source_ms in [812, 1_000, 4_001, 6_730] {
            let output = mapping.source_to_output_ms(source_ms).unwrap();
            assert_eq!(mapping.output_to_source_ms(output).unwrap(), source_ms);
        }
    }
}
