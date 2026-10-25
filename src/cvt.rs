//! CVT set-point temperatures, not a thermodynamic or transport simulation.
use thiserror::Error;

pub const ABSOLUTE_ZERO_C: f64 = -273.15;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{0}")]
pub struct TemperatureError(pub String);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CvtConditions {
    source_c: f64,
    sink_c: f64,
}

impl CvtConditions {
    pub fn new(source_c: f64, sink_c: f64) -> Result<Self, TemperatureError> {
        for (label, value) in [("Source", source_c), ("Sink", sink_c)] {
            if !value.is_finite() {
                return Err(TemperatureError(format!(
                    "{label} temperature must be a finite number in °C."
                )));
            }
            if value < ABSOLUTE_ZERO_C {
                return Err(TemperatureError(format!(
                    "{label} temperature cannot be below absolute zero (−273.15 °C)."
                )));
            }
        }
        Ok(Self { source_c, sink_c })
    }

    pub fn parse(source: &str, sink: &str) -> Result<Self, TemperatureError> {
        let parse = |label, text: &str| {
            text.trim().parse::<f64>().map_err(|_| {
                TemperatureError(format!(
                    "Enter a {label} temperature in °C, using a decimal point if needed."
                ))
            })
        };
        Self::new(parse("source", source)?, parse("sink", sink)?)
    }

    pub fn source_c(self) -> f64 {
        self.source_c
    }
    pub fn sink_c(self) -> f64 {
        self.sink_c
    }

    /// Signed difference. Equal and reversed temperature arrangements are valid.
    pub fn delta_c(self) -> f64 {
        self.source_c - self.sink_c
    }
}

/// One common scale for every displayed experiment, including isothermal pairs.
/// Normalise in f64 before converting to screen coordinates to avoid f32 overflow.
#[derive(Clone, Copy, Debug)]
pub struct TemperatureScale {
    pub min_c: f64,
    pub max_c: f64,
}

impl TemperatureScale {
    pub fn for_conditions(conditions: impl IntoIterator<Item = CvtConditions>) -> Self {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for pair in conditions {
            min = min.min(pair.source_c()).min(pair.sink_c());
            max = max.max(pair.source_c()).max(pair.sink_c());
        }
        if !min.is_finite() {
            return Self {
                min_c: 0.0,
                max_c: 1000.0,
            };
        }
        let span = max - min;
        let padding = if span > 0.0 {
            (span * 0.18).max(1.0)
        } else {
            max.abs().max(100.0) * 0.1
        };
        let low = (min - padding).max(ABSOLUTE_ZERO_C);
        let high = max + padding;
        Self {
            min_c: low,
            max_c: if high.is_finite() { high } else { max },
        }
    }

    pub fn fraction(self, temperature_c: f64) -> f64 {
        ((temperature_c - self.min_c) / (self.max_c - self.min_c)).clamp(0.0, 1.0)
    }

    pub fn ticks(self) -> Vec<f64> {
        let raw_step = (self.max_c - self.min_c) / 5.0;
        let magnitude = 10_f64.powf(raw_step.log10().floor());
        let size = raw_step / magnitude;
        let step = magnitude
            * if size <= 1.0 {
                1.0
            } else if size <= 2.0 {
                2.0
            } else if size <= 5.0 {
                5.0
            } else {
                10.0
            };
        let first = (self.min_c / step).ceil() * step;
        (0..=8)
            .map(|i| first + f64::from(i) * step)
            .take_while(|value| value.is_finite() && *value <= self.max_c)
            .collect()
    }
}

/// Compact display only; storage retains the entered IEEE 754 values.
pub fn temperature_text(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    if value.abs() >= 1_000_000.0 || value.abs() < 0.000001 {
        return format!("{value:.6e}");
    }
    format!("{value:.6}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .into()
}
