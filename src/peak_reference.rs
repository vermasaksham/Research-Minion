//! EDX photon energies and XRD diffraction angles have distinct types and units.
use crate::peak::{self, PeakError};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::OnceLock};

pub const HC_EV_A: f64 = 12398.419843320026;
pub const REFERENCE_CALCULATION: &str = "edx-direct-and-cif-metric-bragg/1";
type Result<T> = std::result::Result<T, PeakError>;

fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(PeakError(message.into()))
    }
}
fn positive(value: f64, label: &str) -> Result<f64> {
    require(
        value.is_finite() && value > 0.,
        &format!("{label} must be finite and greater than zero."),
    )?;
    Ok(value)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Element {
    pub symbol: String,
    pub name: String,
    pub atomic_number: u8,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EmissionLine {
    pub id: String,
    pub element: String,
    pub label: String,
    pub transition: String,
    pub source_transitions: Vec<String>,
    pub family: String,
    pub energy_ev: f64,
    pub uncertainty_ev: f64,
    pub reported_energy_ev: String,
    pub reported_uncertainty_ev: String,
    pub mass_number: String,
    pub blend: bool,
    pub shell: String,
    pub edge_ev: f64,
    pub edge_uncertainty_ev: f64,
    pub edge_reported_ev: String,
    pub edge_reported_uncertainty_ev: String,
    pub measurement: String,
    pub reference_codes: String,
    pub source: String,
    pub source_file: String,
    pub note: String,
}
impl EmissionLine {
    pub fn energy_kev(&self) -> f64 {
        self.energy_ev / 1000.
    }
    pub fn energy_text_kev(&self) -> String {
        let digits = self
            .reported_energy_ev
            .split_once('.')
            .map_or(3, |(_, fraction)| fraction.len() + 3);
        format!("{:.*}", digits.min(12), self.energy_kev())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Source {
    pub file: String,
    pub retrieved_utc: String,
    pub url: String,
    pub sha256: String,
    pub canonical_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct CrystalCell {
    pub lengths_a: [f64; 3],
    pub angles_deg: [f64; 3],
}
impl CrystalCell {
    pub fn validate(&self) -> Result<()> {
        for length in self.lengths_a {
            positive(length, "Cell length (Å)")?;
        }
        for angle in self.angles_deg {
            require(
                angle.is_finite() && angle > 0. && angle < 180.,
                "Cell angles must lie strictly between 0 and 180 degrees.",
            )?;
        }
        let [ca, cb, cg] = self.angles_deg.map(|angle| angle.to_radians().cos());
        let determinant = 1. + 2. * ca * cb * cg - ca * ca - cb * cb - cg * cg;
        require(
            determinant.is_finite() && determinant > 1e-12,
            "Cell metric is degenerate or outside the supported numeric range.",
        )
    }
    pub fn d_spacing(&self, hkl: [i32; 3]) -> Result<f64> {
        self.validate()?;
        require(hkl != [0, 0, 0], "(0 0 0) is not a reflection.")?;
        let [ca, cb, cg] = self.angles_deg.map(|angle| angle.to_radians().cos());
        let det = 1. + 2. * ca * cb * cg - ca * ca - cb * cb - cg * cg;
        let v = [
            f64::from(hkl[0]) / self.lengths_a[0],
            f64::from(hkl[1]) / self.lengths_a[1],
            f64::from(hkl[2]) / self.lengths_a[2],
        ];
        let inverse_d_squared = ((1. - ca * ca) * v[0] * v[0]
            + (1. - cb * cb) * v[1] * v[1]
            + (1. - cg * cg) * v[2] * v[2]
            + 2. * (ca * cb - cg) * v[0] * v[1]
            + 2. * (ca * cg - cb) * v[0] * v[2]
            + 2. * (cb * cg - ca) * v[1] * v[2])
            / det;
        positive(inverse_d_squared, "Reciprocal metric")?;
        positive(1. / inverse_d_squared.sqrt(), "d-spacing (Å)")
    }
    /// (a', b', c')=(b,c,a), with angles permuted consistently.
    pub fn cycle_axes(self) -> Self {
        Self {
            lengths_a: [self.lengths_a[1], self.lengths_a[2], self.lengths_a[0]],
            angles_deg: [self.angles_deg[1], self.angles_deg[2], self.angles_deg[0]],
        }
    }
}
pub fn pbnm_to_pnma([h, k, l]: [i32; 3]) -> [i32; 3] {
    [k, l, h]
}
pub fn photon_wavelength_a(energy_ev: f64) -> Result<f64> {
    positive(
        HC_EV_A / positive(energy_ev, "Photon energy (eV)")?,
        "Photon wavelength (Å)",
    )
}
/// Necessary threshold for direct electron ionisation, never a count prediction.
pub fn above_shell_edge(beam_kv: f64, edge_ev: f64) -> Result<bool> {
    let beam_ev = positive(beam_kv, "Beam voltage (kV)")? * 1000.;
    positive(beam_ev, "Beam energy (eV)")?;
    Ok(beam_ev > positive(edge_ev, "Shell ionisation edge (eV)")?)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReferenceReflection {
    pub id: String,
    pub hkl: [i32; 3],
    pub d_a: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Phase {
    pub id: String,
    pub label: String,
    pub name: String,
    pub cod: String,
    pub space_group: String,
    pub system: String,
    pub lengths_a: [f64; 3],
    pub angles_deg: [f64; 3],
    pub cell_reported: [String; 3],
    pub citation: String,
    pub doi: String,
    pub source: String,
    pub source_file: String,
    pub sha256: String,
    pub cif_text: String,
    pub reflections: Vec<ReferenceReflection>,
}
impl Phase {
    pub fn cell(&self) -> CrystalCell {
        CrystalCell {
            lengths_a: self.lengths_a,
            angles_deg: self.angles_deg,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Radiation {
    pub id: String,
    pub label: String,
    pub energy_ev: f64,
    pub uncertainty_ev: f64,
    pub wavelength_a: f64,
    pub source: String,
    pub reported_energy_ev: String,
    pub reported_uncertainty_ev: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Catalog {
    pub schema: u32,
    pub version: String,
    pub checked: String,
    pub hc_ev_a: f64,
    pub elements: Vec<Element>,
    pub edx: Vec<EmissionLine>,
    pub phases: Vec<Phase>,
    pub radiation: Vec<Radiation>,
    pub sources: Vec<Source>,
}

impl Catalog {
    pub fn parse(text: &str) -> Result<Self> {
        let value: Self = serde_json::from_str(text)
            .map_err(|e| PeakError(format!("Reference data could not load: {e}")))?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        require(
            self.schema == 1 && self.hc_ev_a == HC_EV_A,
            "Unsupported reference schema or constants.",
        )?;
        let mut ids = BTreeSet::new();
        for line in &self.edx {
            require(ids.insert(&line.id), "Duplicate EDX reference ID.")?;
            require(
                self.elements.iter().any(|e| e.symbol == line.element),
                "Unknown EDX element.",
            )?;
            for value in [
                line.energy_ev,
                line.uncertainty_ev,
                line.edge_ev,
                line.edge_uncertainty_ev,
            ] {
                positive(value, "EDX reference value")?;
            }
            require(
                line.energy_ev < line.edge_ev && line.measurement == "NIST direct experimental",
                "Invalid EDX energy or measurement provenance.",
            )?;
            require(
                line.reported_energy_ev.parse::<f64>().ok() == Some(line.energy_ev)
                    && line.reported_uncertainty_ev.parse::<f64>().ok()
                        == Some(line.uncertainty_ev),
                "Reported EDX values disagree with numeric values.",
            )?;
            require(
                self.sources.iter().any(|s| s.file == line.source_file),
                "Missing EDX source.",
            )?;
        }
        ids.clear();
        for phase in &self.phases {
            require(ids.insert(&phase.id), "Duplicate XRD phase ID.")?;
            phase.cell().validate()?;
            require(
                self.sources
                    .iter()
                    .any(|s| s.file == phase.source_file && s.sha256 == phase.sha256),
                "Missing or inconsistent CIF provenance.",
            )?;
            let mut reflections = BTreeSet::new();
            for row in &phase.reflections {
                require(
                    reflections.insert(row.hkl),
                    "Duplicate reference reflection.",
                )?;
                let d = phase.cell().d_spacing(row.hkl)?;
                require(
                    row.d_a.is_finite() && (d - row.d_a).abs() < 1e-10,
                    "Reference d-spacing disagrees with the CIF cell metric.",
                )?;
            }
        }
        for radiation in &self.radiation {
            positive(radiation.uncertainty_ev, "Radiation uncertainty")?;
            require(
                (photon_wavelength_a(radiation.energy_ev)? - radiation.wavelength_a).abs() < 1e-12,
                "Radiation wavelength disagrees with its photon energy.",
            )?;
        }
        Ok(())
    }
}

pub fn catalog() -> Result<&'static Catalog> {
    static CATALOG: OnceLock<Result<Catalog>> = OnceLock::new();
    CATALOG
        .get_or_init(|| Catalog::parse(include_str!("../data/peak/catalog.json")))
        .as_ref()
        .map_err(Clone::clone)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiffractionPosition {
    pub id: String,
    pub phase_id: String,
    pub phase_label: String,
    pub source_hkl: [i32; 3],
    pub displayed_hkl: [i32; 3],
    pub setting: String,
    pub d_a: f64,
    pub two_theta_deg: Option<f64>,
}

pub fn diffraction_positions(
    catalog: &Catalog,
    phases: &[String],
    wavelength_a: f64,
    pnma: bool,
) -> Result<Vec<DiffractionPosition>> {
    positive(wavelength_a, "Wavelength (Å)")?;
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    for id in phases {
        require(seen.insert(id), "Choose each phase only once.")?;
        let phase = catalog
            .phases
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(|| PeakError(format!("Unknown phase: {id}")))?;
        let mapped = pnma && id == "sb2se3";
        let cell = if mapped {
            phase.cell().cycle_axes()
        } else {
            phase.cell()
        };
        for reference in &phase.reflections {
            let hkl = if mapped {
                pbnm_to_pnma(reference.hkl)
            } else {
                reference.hkl
            };
            let d = cell.d_spacing(hkl)?;
            result.push(DiffractionPosition {
                id: reference.id.clone(),
                phase_id: id.clone(),
                phase_label: phase.label.clone(),
                source_hkl: reference.hkl,
                displayed_hkl: hkl,
                setting: if mapped {
                    "Pnma (mapped from source Pbnm)".into()
                } else {
                    phase.space_group.clone()
                },
                d_a: d,
                two_theta_deg: peak::two_theta_deg(wavelength_a, d)?,
            });
        }
    }
    result.sort_by(|a, b| {
        a.two_theta_deg
            .unwrap_or(f64::INFINITY)
            .total_cmp(&b.two_theta_deg.unwrap_or(f64::INFINITY))
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(result)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExplorerParameters {
    pub elements: Vec<String>,
    pub phases: Vec<String>,
    pub family: Option<String>,
    pub beam_kv: f64,
    pub wavelength_a: f64,
    pub radiation_label: String,
    pub pnma: bool,
    pub energy_max_kev: f64,
    pub angle_min_deg: f64,
    pub angle_max_deg: f64,
}
impl Default for ExplorerParameters {
    fn default() -> Self {
        Self {
            elements: vec!["Sb".into(), "Se".into()],
            phases: vec!["sb2se3".into()],
            family: None,
            beam_kv: 20.,
            wavelength_a: HC_EV_A / 8047.8227,
            radiation_label: "Cu Kα₁ (NIST direct experimental; vacuum wavelength)".into(),
            pnma: false,
            energy_max_kev: 15.,
            angle_min_deg: 10.,
            angle_max_deg: 90.,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnergyReferences {
    pub quantity: String,
    pub energy_unit: String,
    pub markers: Vec<EmissionLine>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiffractionReferences {
    pub quantity: String,
    pub angle_unit: String,
    pub length_unit: String,
    pub phases: Vec<Phase>,
    pub positions: Vec<DiffractionPosition>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReferenceSnapshot {
    pub schema: u32,
    pub application_version: String,
    pub calculation_version: String,
    pub catalog_version: String,
    pub created_utc: String,
    pub parameters: ExplorerParameters,
    pub edx: EnergyReferences,
    pub xrd: DiffractionReferences,
    pub sources: Vec<Source>,
    pub interpretation: String,
    pub scope: String,
}

pub fn snapshot(
    catalog: &Catalog,
    parameters: ExplorerParameters,
    interpretation: &str,
) -> Result<ReferenceSnapshot> {
    positive(parameters.beam_kv, "Beam voltage (kV)")?;
    positive(parameters.energy_max_kev, "EDX energy window (keV)")?;
    require(
        parameters.angle_min_deg.is_finite()
            && parameters.angle_max_deg.is_finite()
            && parameters.angle_min_deg >= 0.
            && parameters.angle_min_deg < parameters.angle_max_deg
            && parameters.angle_max_deg <= 180.,
        "XRD angle window must satisfy 0 ≤ minimum < maximum ≤ 180°.",
    )?;
    require(
        parameters
            .family
            .as_deref()
            .is_none_or(|f| f == "K" || f == "L"),
        "Unsupported EDX line family.",
    )?;
    let mut seen = BTreeSet::new();
    for element in &parameters.elements {
        require(
            seen.insert(element) && catalog.elements.iter().any(|e| &e.symbol == element),
            "Unknown or duplicate selected element.",
        )?;
    }
    let edx = catalog
        .edx
        .iter()
        .filter(|line| {
            parameters.elements.contains(&line.element)
                && parameters
                    .family
                    .as_ref()
                    .is_none_or(|family| family == &line.family)
        })
        .cloned()
        .collect();
    let positions = diffraction_positions(
        catalog,
        &parameters.phases,
        parameters.wavelength_a,
        parameters.pnma,
    )?;
    let phases = catalog
        .phases
        .iter()
        .filter(|phase| parameters.phases.contains(&phase.id))
        .cloned()
        .collect();
    Ok(ReferenceSnapshot {
        schema: 1, application_version: env!("CARGO_PKG_VERSION").into(), calculation_version: REFERENCE_CALCULATION.into(),
        catalog_version: catalog.version.clone(), created_utc: chrono::Utc::now().to_rfc3339(), parameters,
        edx: EnergyReferences { quantity: "emitted characteristic photon energy".into(), energy_unit: "eV".into(), markers: edx },
        xrd: DiffractionReferences { quantity: "calculated angle-dispersive diffraction position".into(),
            angle_unit: "degree_2theta".into(), length_unit: "angstrom".into(), phases, positions },
        sources: catalog.sources.clone(), interpretation: interpretation.into(),
        scope: "Selected reference positions, not measured spectra. Marker height is not intensity. EDX identifies elemental line families, not crystalline phases. XRD positions use cited CIF cell metrics and n=1 Bragg law; no intensity or phase-fraction prediction. EDX energy and XRD angle are distinct physical quantities. Beam/edge comparison is only a necessary direct-ionisation condition, not a useful-count prediction.".into(),
    })
}

impl ReferenceSnapshot {
    pub fn json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| PeakError(format!("Could not encode reference snapshot: {e}")))
    }
    pub fn report(&self) -> Result<String> {
        let mut text = format!(
            "# Peak Explorer · lab reference\n\nEDX photon energy (keV) ≠ XRD scattering angle (degrees 2θ).\n\nReference catalog: {} · Calculation: {}\n\n## EDX · emitted photon energies\n\nBeam: {} kV. Values below are NIST direct experimental energies.\n\n| Element / line | Transition | E (keV) | Reported uncertainty (eV) |\n|---|---|---|---|\n",
            self.catalog_version, self.calculation_version, self.parameters.beam_kv
        );
        for line in &self.edx.markers {
            text.push_str(&format!(
                "| {} {} | {} | {} | ±{} |\n",
                line.element,
                line.label,
                line.transition,
                line.energy_text_kev(),
                line.reported_uncertainty_ev
            ));
        }
        text.push_str(&format!("\n## XRD · calculated diffraction angles\n\n{} · λ = {} Å · n = 1\n\n| Phase | h k l (displayed setting) | d (Å) | 2θ (degrees) |\n|---|---|---|---|\n", self.parameters.radiation_label, self.parameters.wavelength_a));
        for row in &self.xrd.positions {
            text.push_str(&format!(
                "| {} | {} {} {} | {} | {} |\n",
                row.phase_label,
                row.displayed_hkl[0],
                row.displayed_hkl[1],
                row.displayed_hkl[2],
                row.d_a,
                row.two_theta_deg
                    .map_or_else(|| "No first-order Bragg solution".into(), |v| v.to_string())
            ));
        }
        text.push_str("\n## Sources\n\n");
        for phase in &self.xrd.phases {
            text.push_str(&format!(
                "- {}: {} · COD {} · {} · {}\n",
                phase.label, phase.citation, phase.cod, phase.space_group, phase.source
            ));
        }
        for symbol in &self.parameters.elements {
            text.push_str(&format!("- NIST {symbol}: https://physics.nist.gov/cgi-bin/XrayTrans/search.pl?element={symbol}&trans=All&units=eV\n"));
        }
        text.push_str(&format!("\n## Researcher interpretation\n\n{}\n\n## Scope\n\n{}\n\n## Reproducible snapshot\n\nThe JSON retains all selected references, units, source hashes and crystal settings.\n\n```json\n{}\n```\n", self.interpretation, self.scope, self.json()?));
        Ok(text)
    }
}

/// Exports remain inside the active profile; create_new prevents silent overwrites.
pub fn export_to_profile(
    snapshot: &ReferenceSnapshot,
    profile_dir: &std::path::Path,
) -> Result<std::path::PathBuf> {
    use std::io::Write;
    let json = snapshot.json()?;
    let directory = profile_dir.join("exports");
    std::fs::create_dir_all(&directory)
        .map_err(|e| PeakError(format!("Could not create export folder: {e}")))?;
    let path = directory.join(format!("peak-references-{}.json", uuid::Uuid::new_v4()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| PeakError(format!("Could not create export file: {e}")))?;
    file.write_all(json.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| PeakError(format!("Could not finish export: {e}")))?;
    Ok(path)
}
