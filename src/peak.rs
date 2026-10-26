//! First-order Bragg and user-entered orthorhombic reflection positions.
//! No bundled phase catalog, symmetry filtering, intensities or phase identification.
use thiserror::Error;

pub const CALCULATION_VERSION: &str = "orthorhombic-bragg/1";

#[derive(Clone, Debug, Error, PartialEq)]
#[error("{0}")]
pub struct PeakError(pub String);

type Result<T> = std::result::Result<T, PeakError>;

fn positive(value: f64, label: &str) -> Result<f64> {
    if !value.is_finite() || value <= 0.0 {
        return Err(PeakError(format!(
            "{label} must be finite and greater than zero."
        )));
    }
    Ok(value)
}

/// n=1, wavelength and d in the same length unit. None means no physical
/// first-order solution for this wavelength (lambda > 2d).
pub fn two_theta_deg(wavelength_a: f64, d_a: f64) -> Result<Option<f64>> {
    positive(wavelength_a, "Wavelength (Å)")?;
    positive(d_a, "d-spacing (Å)")?;
    let ratio = (wavelength_a / d_a) / 2.0;
    if ratio > 1.0 {
        return Ok(None);
    }
    if ratio == 0.0 {
        return Err(PeakError(
            "Inputs exceed the supported numeric range.".into(),
        ));
    }
    Ok(Some(2.0 * ratio.asin().to_degrees()))
}

pub fn d_spacing_a(wavelength_a: f64, two_theta: f64) -> Result<f64> {
    positive(wavelength_a, "Wavelength (Å)")?;
    if !two_theta.is_finite() || two_theta <= 0.0 || two_theta > 180.0 {
        return Err(PeakError(
            "2θ must be greater than 0 and at most 180 degrees.".into(),
        ));
    }
    let d = (wavelength_a / 2.0) / (two_theta.to_radians() / 2.0).sin();
    positive(d, "Calculated d-spacing")
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrthorhombicCell {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

impl OrthorhombicCell {
    pub fn new(a: f64, b: f64, c: f64) -> Result<Self> {
        Ok(Self {
            a: positive(a, "a (Å)")?,
            b: positive(b, "b (Å)")?,
            c: positive(c, "c (Å)")?,
        })
    }

    pub fn d_spacing(self, hkl: [i32; 3]) -> Result<f64> {
        // Validate even when callers construct the public struct directly.
        Self::new(self.a, self.b, self.c)?;
        if hkl == [0, 0, 0] {
            return Err(PeakError("(0 0 0) is not a reflection.".into()));
        }
        let inverse_d = (f64::from(hkl[0]) / self.a)
            .hypot(f64::from(hkl[1]) / self.b)
            .hypot(f64::from(hkl[2]) / self.c);
        positive(1.0 / inverse_d, "Calculated d-spacing")
    }
}

/// One explicit h k l per line, optional commas; no automatic reflection generation.
pub fn parse_reflections(text: &str) -> Result<Vec<[i32; 3]>> {
    let mut result = Vec::new();
    for (line_number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if result.len() >= 500 {
            return Err(PeakError("Enter at most 500 reflections.".into()));
        }
        let line = line.replace(',', " ");
        let parts: Vec<_> = line.split_whitespace().collect();
        let invalid = || {
            PeakError(format!(
                "Line {}: enter three signed integers h k l, excluding 0 0 0.",
                line_number + 1
            ))
        };
        if parts.len() != 3 {
            return Err(invalid());
        }
        let mut hkl = [0; 3];
        for (value, part) in hkl.iter_mut().zip(parts) {
            *value = part.parse().map_err(|_| invalid())?;
        }
        if hkl == [0, 0, 0] {
            return Err(invalid());
        }
        if result.contains(&hkl) {
            return Err(PeakError(format!(
                "Line {} repeats an h k l already entered.",
                line_number + 1
            )));
        }
        result.push(hkl);
    }
    if result.is_empty() {
        return Err(PeakError("Enter at least one h k l reflection.".into()));
    }
    Ok(result)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reflection {
    pub hkl: [i32; 3],
    pub d_a: f64,
    pub two_theta: Option<f64>,
}

pub fn calculate(
    cell: OrthorhombicCell,
    wavelength_a: f64,
    indices: &[[i32; 3]],
) -> Result<Vec<Reflection>> {
    positive(wavelength_a, "Wavelength (Å)")?;
    indices
        .iter()
        .map(|hkl| {
            let d_a = cell.d_spacing(*hkl)?;
            Ok(Reflection {
                hkl: *hkl,
                d_a,
                two_theta: two_theta_deg(wavelength_a, d_a)?,
            })
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct Analysis {
    pub cell: OrthorhombicCell,
    pub wavelength_a: f64,
    pub results: Vec<Reflection>,
    pub material: String,
    /// User-supplied axis/space-group convention; no automatic setting conversion.
    pub setting: String,
    pub source: String,
    pub interpretation: String,
}

impl Analysis {
    /// Self-contained readable snapshot, frozen on save; never a reference to mutable UI state.
    pub fn report(&self) -> String {
        let mut text = format!(
            "Peak Explorer — user-entered orthorhombic XRD\nResearch Minion {}\nCalculation: {}\nCalculated: {} UTC\nMaterial/phase label: {}\nAxis/structure setting (as entered): {}\nCell source/provenance (as entered): {}\na = {} Å; b = {} Å; c = {} Å\nα = β = γ = 90°\nWavelength = {} Å (user-entered); Bragg order n = 1\n1/d² = h²/a² + k²/b² + l²/c²; 2θ = 2 asin(λ/(2d))\n\nSelected h k l, d (Å), 2θ (degrees):\n",
            env!("CARGO_PKG_VERSION"),
            CALCULATION_VERSION,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
            self.material,
            self.setting,
            self.source,
            self.cell.a,
            self.cell.b,
            self.cell.c,
            self.wavelength_a
        );
        for row in &self.results {
            let angle = row
                .two_theta
                .map(|value| value.to_string())
                .unwrap_or_else(|| "No first-order Bragg solution (λ > 2d)".into());
            text.push_str(&format!(
                "{} {} {}\t{}\t{}\n",
                row.hkl[0], row.hkl[1], row.hkl[2], row.d_a, angle
            ));
        }
        text.push_str("\nLimitations: positions only for entered reflections; no symmetry-absence filtering, intensities, phase identification or phase fractions. Orthorhombic axes are used exactly as entered; no Pbnm/Pnma conversion. This manual calculation does not use the bundled reference catalog.\n\nResearcher interpretation:\n");
        text.push_str(&self.interpretation);
        text
    }
}
