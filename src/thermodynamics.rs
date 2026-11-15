//! Pure thermodynamic reaction arithmetic. No database, UI or network calls.
//!
//! The installed reference catalog currently stores point values. Its records
//! must be used with `HeatModel::ReferenceOnly`. Constant-Cp integration is an
//! explicit engine input for independently validated models, never inferred
//! from a Cp value entered at one temperature.
use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::chemistry::ELEMENTS;

/// Molar gas constant, J/(mol K). CODATA 2018 value from exact SI constants.
pub const GAS_CONSTANT_J_MOL_K: f64 = 8.314_462_618_153_24;
pub const FARADAY_C_MOL: f64 = 96_485.332_12;

#[derive(Debug, Error, PartialEq)]
pub enum EngineError {
    #[error("{0}")]
    Invalid(String),
    #[error("The required species record '{0}' is missing.")]
    MissingSpecies(String),
    #[error("Record '{0}' has not been verified against its source.")]
    Unverified(String),
    #[error("Record '{0}' belongs to a different or disabled source database.")]
    SourceMismatch(String),
    #[error("Record '{0}' has no standard enthalpy of formation or molar entropy.")]
    MissingValues(String),
    #[error("Record '{0}' is outside its validated temperature range at {1} K.")]
    OutsideRange(String, f64),
    #[error("Record '{0}' is defined only at its reference temperature of {1} K.")]
    ReferenceOnly(String, f64),
    #[error("Reaction standard pressures differ; no conversion has been specified.")]
    MixedPressure,
    #[error("The reaction is not element-balanced: {0}.")]
    Unbalanced(String),
    #[error("A calculation overflowed; review the inputs and units.")]
    Overflow,
}
pub type Result<T> = std::result::Result<T, EngineError>;

/// Exact signed stoichiometric coefficient. Negative = reactant, positive = product.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    pub fn new(numerator: i64, denominator: i64) -> Result<Self> {
        if denominator == 0 {
            return Err(EngineError::Invalid(
                "Stoichiometric denominator must be nonzero.".into(),
            ));
        }
        Self::from_parts(i128::from(numerator), i128::from(denominator))
    }
    pub fn numerator(self) -> i128 {
        self.numerator
    }
    pub fn denominator(self) -> i128 {
        self.denominator
    }
    fn from_parts(numerator: i128, denominator: i128) -> Result<Self> {
        if denominator == 0 {
            return Err(EngineError::Overflow);
        }
        let (numerator, denominator) = if denominator < 0 {
            (
                numerator.checked_neg().ok_or(EngineError::Overflow)?,
                denominator.checked_neg().ok_or(EngineError::Overflow)?,
            )
        } else {
            (numerator, denominator)
        };
        let divisor = gcd(numerator.unsigned_abs(), denominator as u128);
        let divisor = i128::try_from(divisor).map_err(|_| EngineError::Overflow)?;
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }
    fn checked_add(self, other: Self) -> Result<Self> {
        let numerator = self
            .numerator
            .checked_mul(other.denominator)
            .and_then(|a| {
                other
                    .numerator
                    .checked_mul(self.denominator)
                    .and_then(|b| a.checked_add(b))
            })
            .ok_or(EngineError::Overflow)?;
        let denominator = self
            .denominator
            .checked_mul(other.denominator)
            .ok_or(EngineError::Overflow)?;
        Self::from_parts(numerator, denominator)
    }
    fn checked_mul_u64(self, value: u64) -> Result<Self> {
        Self::from_parts(
            self.numerator
                .checked_mul(i128::from(value))
                .ok_or(EngineError::Overflow)?,
            self.denominator,
        )
    }
    fn as_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}
fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let rest = a % b;
        a = b;
        b = rest;
    }
    a
}

#[derive(Clone, Debug, PartialEq)]
pub enum HeatModel {
    /// H°f and S° are valid only at `reference_k`.
    ReferenceOnly,
    /// Cp° is explicitly assumed constant over the record's validated range.
    ConstantCp { joule_per_mol_kelvin: f64 },
    /// Cp°(T) = a + bT + c/T² + dT², with T in K and Cp in J/(mol K).
    PolynomialCp { a: f64, b: f64, c: f64, d: f64 },
}

/// Owned input snapshot. This contains the source and revision actually used.
#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesRecord {
    pub id: String,
    pub source_id: String,
    pub source_name: String,
    pub source_authors: String,
    pub source_year: i32,
    pub source_reference: String,
    pub source_enabled: bool,
    pub source_locator: String,
    pub species_name: String,
    pub comments: String,
    pub imported_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub revision: i64,
    pub verified_at: Option<DateTime<Utc>>,
    pub formula: String,
    pub phase: String,
    pub min_k: f64,
    pub max_k: f64,
    pub reference_k: f64,
    pub reference_pressure_bar: f64,
    pub formation_enthalpy_j_mol: Option<f64>,
    pub molar_entropy_j_mol_k: Option<f64>,
    pub model: HeatModel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReactionTerm {
    pub species_id: String,
    pub coefficient: Rational,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reaction {
    pub name: String,
    pub terms: Vec<ReactionTerm>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UsedSpecies {
    pub coefficient: Rational,
    pub record: SpeciesRecord,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Calculation {
    pub reaction: Reaction,
    pub temperature_k: f64,
    pub standard_pressure_bar: f64,
    pub source_id: String,
    pub delta_h_j_mol: f64,
    pub delta_s_j_mol_k: f64,
    pub delta_g_j_mol: f64,
    pub ln_k: f64,
    pub log10_k: f64,
    /// None when the exponential cannot be represented as a positive finite f64.
    pub equilibrium_constant: Option<f64>,
    pub species_used: Vec<UsedSpecies>,
}

/// A formula-based reaction can select a different phase record at each temperature.
#[derive(Clone, Debug, PartialEq)]
pub struct PhaseReactionTerm {
    pub formula: String,
    pub phase: Option<String>,
    pub coefficient: Rational,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseReaction {
    pub name: String,
    pub terms: Vec<PhaseReactionTerm>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PhaseMode {
    Automatic,
    /// Formula to forced phase code (s, l or g).
    Fixed(BTreeMap<String, String>),
    /// Every term must supply its phase.
    Explicit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseChange {
    pub formula: String,
    pub from_phase: String,
    pub to_phase: String,
    pub from_record_id: String,
    pub to_record_id: String,
    pub between_k: (f64, f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseSeries {
    pub points: Vec<Calculation>,
    pub changes: Vec<PhaseChange>,
}

/// Parses `2/3 SbCl3(g) -> 2/3 Sb + Cl2(g)`; positive coefficients are products.
pub fn parse_reaction(name: &str, input: &str) -> Result<PhaseReaction> {
    let (left, right) = input
        .split_once("->")
        .ok_or_else(|| EngineError::Invalid("Use -> between reactants and products.".into()))?;
    if right.contains("->") || name.trim().is_empty() {
        return Err(EngineError::Invalid(
            "Enter one named reaction with one -> arrow.".into(),
        ));
    }
    let mut terms = Vec::new();
    for (side, sign) in [(left, -1), (right, 1)] {
        for part in side.split('+') {
            let part = part.trim();
            if part.is_empty() {
                return Err(EngineError::Invalid("A reaction term is empty.".into()));
            }
            let (magnitude, species) = match part.split_once(char::is_whitespace) {
                Some((coefficient, species))
                    if coefficient.chars().all(|c| c.is_ascii_digit() || c == '/') =>
                {
                    let (numerator, denominator) =
                        coefficient.split_once('/').unwrap_or((coefficient, "1"));
                    let numerator = numerator
                        .parse::<i64>()
                        .map_err(|_| EngineError::Invalid("Invalid coefficient.".into()))?;
                    let denominator = denominator
                        .parse::<i64>()
                        .map_err(|_| EngineError::Invalid("Invalid coefficient.".into()))?;
                    (
                        Rational::new(numerator * sign, denominator)?,
                        species.trim(),
                    )
                }
                _ => (Rational::new(sign, 1)?, part),
            };
            let (formula, phase) = if species.ends_with(')') {
                match species.rsplit_once('(') {
                    Some((formula, phase_with_close))
                        if ["s)", "l)", "g)"].contains(&phase_with_close) =>
                    {
                        (formula.trim(), Some(phase_with_close[..1].to_string()))
                    }
                    _ => (species, None),
                }
            } else {
                (species, None)
            };
            parse_formula(formula)?;
            terms.push(PhaseReactionTerm {
                formula: formula.into(),
                phase,
                coefficient: magnitude,
            });
        }
    }
    Ok(PhaseReaction {
        name: name.trim().into(),
        terms,
    })
}

/// For an ideal-gas equilibrium with a specified activity for every *other* gas.
/// Returns p/p° (dimensionless) and p in bar at the recorded standard pressure.
pub fn equilibrium_gas_pressure(
    result: &Calculation,
    target_species_id: &str,
    other_gas_activities: &BTreeMap<String, f64>,
) -> Result<(f64, f64)> {
    let target = result
        .species_used
        .iter()
        .find(|used| used.record.id == target_species_id && used.record.phase == "g")
        .ok_or_else(|| EngineError::Invalid("Select a gas species in the reaction.".into()))?;
    let mut ln_other = 0.0;
    for used in &result.species_used {
        if used.record.phase != "g" || used.record.id == target_species_id {
            continue;
        }
        let activity = *other_gas_activities.get(&used.record.id).ok_or_else(|| {
            EngineError::Invalid(format!(
                "Enter the activity p/p° of {}.",
                used.record.formula
            ))
        })?;
        if !activity.is_finite() || activity <= 0.0 {
            return Err(EngineError::Invalid(format!(
                "The activity of {} must be positive and finite.",
                used.record.formula
            )));
        }
        ln_other += used.coefficient.as_f64() * activity.ln();
    }
    let ln_p = (result.ln_k - ln_other) / target.coefficient.as_f64();
    let relative = ln_p.exp();
    let pressure = relative * result.standard_pressure_bar;
    if !relative.is_finite() || relative <= 0.0 || !pressure.is_finite() || pressure <= 0.0 {
        return Err(EngineError::Overflow);
    }
    Ok((relative, pressure))
}

/// Standard potential for the reaction as written, requiring explicit electron count.
pub fn standard_electrode_potential(result: &Calculation, electron_count: u32) -> Result<f64> {
    if electron_count == 0 {
        return Err(EngineError::Invalid(
            "Enter the positive number of exchanged electrons z.".into(),
        ));
    }
    Ok(-result.delta_g_j_mol / (f64::from(electron_count) * FARADAY_C_MOL))
}

fn normalized_formula(formula: &str) -> String {
    formula
        .chars()
        .map(|c| match c {
            '₀' => '0',
            '₁' => '1',
            '₂' => '2',
            '₃' => '3',
            '₄' => '4',
            '₅' => '5',
            '₆' => '6',
            '₇' => '7',
            '₈' => '8',
            '₉' => '9',
            c => c,
        })
        .collect()
}

pub fn calculate_phase_aware(
    reaction: &PhaseReaction,
    temperature_k: f64,
    source_id: &str,
    records: &[SpeciesRecord],
    mode: &PhaseMode,
) -> Result<Calculation> {
    if !temperature_k.is_finite() || temperature_k <= 0.0 {
        return Err(EngineError::Invalid(
            "Enter a finite, positive temperature in K.".into(),
        ));
    }
    let mut terms = Vec::with_capacity(reaction.terms.len());
    for term in &reaction.terms {
        let formula = normalized_formula(term.formula.trim());
        parse_formula(&formula)?;
        let requested_phase =
            match mode {
                PhaseMode::Automatic => term.phase.as_deref(),
                PhaseMode::Fixed(forced) => Some(
                    forced
                        .get(&formula)
                        .map(String::as_str)
                        .or(term.phase.as_deref())
                        .ok_or_else(|| {
                            EngineError::Invalid(format!("Choose a fixed phase for {formula}."))
                        })?,
                ),
                PhaseMode::Explicit => Some(term.phase.as_deref().ok_or_else(|| {
                    EngineError::Invalid(format!("Specify a phase for {formula}."))
                })?),
            };
        if requested_phase.is_some_and(|phase| !["s", "l", "g"].contains(&phase)) {
            return Err(EngineError::Invalid(format!(
                "Unsupported phase for {formula}; use s, l or g."
            )));
        }
        let matches: Vec<_> = records
            .iter()
            .filter(|record| {
                record.source_id == source_id
                    && normalized_formula(&record.formula) == formula
                    && requested_phase.is_none_or(|phase| phase == record.phase)
            })
            .collect();
        if matches.is_empty() {
            return Err(EngineError::MissingSpecies(match requested_phase {
                Some(phase) => format!("{formula}({phase}) in {source_id}"),
                None => format!("{formula} in {source_id}"),
            }));
        }
        let applicable: Vec<_> = matches
            .iter()
            .filter(|record| temperature_k >= record.min_k && temperature_k <= record.max_k)
            .collect();
        if applicable.is_empty() {
            return Err(EngineError::Invalid(format!(
                "No {} record for {formula} is valid at {temperature_k} K; calculation stopped without extrapolation.",
                requested_phase.map_or("phase", |phase| phase)
            )));
        }
        if applicable.len() != 1 {
            return Err(EngineError::Invalid(format!(
                "Multiple phase records for {formula} apply at {temperature_k} K; resolve the overlapping ranges before calculation."
            )));
        }
        terms.push(ReactionTerm {
            species_id: applicable[0].id.clone(),
            coefficient: term.coefficient,
        });
    }
    calculate(
        &Reaction {
            name: reaction.name.clone(),
            terms,
        },
        temperature_k,
        source_id,
        records,
    )
}

pub fn calculate_phase_series(
    reaction: &PhaseReaction,
    temperatures_k: &[f64],
    source_id: &str,
    records: &[SpeciesRecord],
    mode: &PhaseMode,
) -> Result<PhaseSeries> {
    if temperatures_k.is_empty()
        || temperatures_k.len() > 10_000
        || temperatures_k.windows(2).any(|w| w[0] >= w[1])
    {
        return Err(EngineError::Invalid(
            "Enter 1–10,000 strictly increasing temperatures in K.".into(),
        ));
    }
    let mut points: Vec<Calculation> = Vec::with_capacity(temperatures_k.len());
    let mut changes = Vec::new();
    for &temperature in temperatures_k {
        let point = calculate_phase_aware(reaction, temperature, source_id, records, mode)
            .map_err(|error| EngineError::Invalid(format!("At {temperature} K: {error}")))?;
        if let Some(previous) = points.last() {
            for (before, after) in previous.species_used.iter().zip(&point.species_used) {
                if before.record.id != after.record.id {
                    changes.push(PhaseChange {
                        formula: before.record.formula.clone(),
                        from_phase: before.record.phase.clone(),
                        to_phase: after.record.phase.clone(),
                        from_record_id: before.record.id.clone(),
                        to_record_id: after.record.id.clone(),
                        between_k: (previous.temperature_k, temperature),
                    });
                }
            }
        }
        points.push(point);
    }
    Ok(PhaseSeries { points, changes })
}

pub fn calculate(
    reaction: &Reaction,
    temperature_k: f64,
    source_id: &str,
    records: &[SpeciesRecord],
) -> Result<Calculation> {
    if !temperature_k.is_finite() || temperature_k <= 0.0 {
        return Err(EngineError::Invalid(
            "Enter a finite, positive temperature in K.".into(),
        ));
    }
    if source_id.trim().is_empty() {
        return Err(EngineError::Invalid("Select a source database.".into()));
    }
    if reaction.name.trim().is_empty() || reaction.terms.len() < 2 {
        return Err(EngineError::Invalid(
            "Name the reaction and enter at least two species.".into(),
        ));
    }
    if !reaction
        .terms
        .iter()
        .any(|term| term.coefficient.numerator < 0)
        || !reaction
            .terms
            .iter()
            .any(|term| term.coefficient.numerator > 0)
    {
        return Err(EngineError::Invalid(
            "Include at least one reactant and one product.".into(),
        ));
    }
    let mut used = Vec::new();
    let mut seen = BTreeSet::new();
    let mut balance: BTreeMap<u8, Rational> = BTreeMap::new();
    let mut pressure = None;
    for term in &reaction.terms {
        if term.coefficient.numerator == 0 || term.coefficient.denominator <= 0 {
            return Err(EngineError::Invalid(
                "Every species needs a nonzero signed stoichiometric coefficient.".into(),
            ));
        }
        if !seen.insert(&term.species_id) {
            return Err(EngineError::Invalid(
                "List each species record once; combine repeated coefficients first.".into(),
            ));
        }
        let record = records
            .iter()
            .find(|record| record.id == term.species_id)
            .ok_or_else(|| EngineError::MissingSpecies(term.species_id.clone()))?;
        validate_record(record, temperature_k, source_id)?;
        if pressure.is_some_and(|previous: f64| previous != record.reference_pressure_bar) {
            return Err(EngineError::MixedPressure);
        }
        pressure = Some(record.reference_pressure_bar);
        for (element, count) in parse_formula(&record.formula)? {
            let current = balance
                .get(&element)
                .copied()
                .unwrap_or(Rational::new(0, 1)?);
            balance.insert(
                element,
                current.checked_add(term.coefficient.checked_mul_u64(count)?)?,
            );
        }
        used.push(UsedSpecies {
            coefficient: term.coefficient,
            record: record.clone(),
        });
    }
    if let Some((&number, _)) = balance.iter().find(|(_, value)| value.numerator != 0) {
        let symbol = ELEMENTS[usize::from(number - 1)].symbol;
        return Err(EngineError::Unbalanced(symbol.into()));
    }
    let mut delta_h = 0.0;
    let mut delta_s = 0.0;
    for item in &used {
        let h = item
            .record
            .formation_enthalpy_j_mol
            .ok_or_else(|| EngineError::MissingValues(item.record.id.clone()))?;
        let s = item
            .record
            .molar_entropy_j_mol_k
            .ok_or_else(|| EngineError::MissingValues(item.record.id.clone()))?;
        let (h, s) = match item.record.model {
            HeatModel::ReferenceOnly => (h, s),
            HeatModel::ConstantCp {
                joule_per_mol_kelvin: cp,
            } => (
                h + cp * (temperature_k - item.record.reference_k),
                s + cp * (temperature_k / item.record.reference_k).ln(),
            ),
            HeatModel::PolynomialCp { a, b, c, d } => {
                let t0 = item.record.reference_k;
                let t = temperature_k;
                (
                    h + a * (t - t0) + b / 2.0 * (t * t - t0 * t0) - c * (1.0 / t - 1.0 / t0)
                        + d / 3.0 * (t.powi(3) - t0.powi(3)),
                    s + a * (t / t0).ln() + b * (t - t0)
                        - c / 2.0 * (1.0 / (t * t) - 1.0 / (t0 * t0))
                        + d / 2.0 * (t * t - t0 * t0),
                )
            }
        };
        let coefficient = item.coefficient.as_f64();
        delta_h += coefficient * h;
        delta_s += coefficient * s;
    }
    let delta_g = delta_h - temperature_k * delta_s;
    let ln_k = -delta_g / (GAS_CONSTANT_J_MOL_K * temperature_k);
    let log10_k = ln_k / std::f64::consts::LN_10;
    if ![delta_h, delta_s, delta_g, ln_k, log10_k]
        .into_iter()
        .all(f64::is_finite)
    {
        return Err(EngineError::Overflow);
    }
    let k = ln_k.exp();
    Ok(Calculation {
        reaction: reaction.clone(),
        temperature_k,
        standard_pressure_bar: pressure
            .ok_or_else(|| EngineError::Invalid("No species.".into()))?,
        source_id: source_id.into(),
        delta_h_j_mol: delta_h,
        delta_s_j_mol_k: delta_s,
        delta_g_j_mol: delta_g,
        ln_k,
        log10_k,
        equilibrium_constant: (k.is_finite() && k > 0.0).then_some(k),
        species_used: used,
    })
}

fn validate_record(record: &SpeciesRecord, temperature_k: f64, source_id: &str) -> Result<()> {
    if record.id.trim().is_empty()
        || record.source_name.trim().is_empty()
        || record.source_authors.trim().is_empty()
        || record.source_year <= 0
        || record.source_reference.trim().is_empty()
        || record.source_locator.trim().is_empty()
        || record.species_name.trim().is_empty()
        || record.formula.trim().is_empty()
        || record.revision < 1
    {
        return Err(EngineError::Invalid(format!(
            "Record '{}' has incomplete provenance.",
            record.id
        )));
    }
    if record.source_id != source_id || !record.source_enabled {
        return Err(EngineError::SourceMismatch(record.id.clone()));
    }
    if !["s", "l", "g"].contains(&record.phase.as_str()) {
        return Err(EngineError::Invalid(format!(
            "Record '{}' has an unsupported phase.",
            record.id
        )));
    }
    if record.verified_at.is_none() {
        return Err(EngineError::Unverified(record.id.clone()));
    }
    for value in [
        record.min_k,
        record.max_k,
        record.reference_k,
        record.reference_pressure_bar,
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(EngineError::Invalid(format!(
                "Record '{}' has invalid temperature or pressure metadata.",
                record.id
            )));
        }
    }
    if record.max_k < record.min_k {
        return Err(EngineError::Invalid(format!(
            "Record '{}' has an invalid range.",
            record.id
        )));
    }
    if temperature_k < record.min_k || temperature_k > record.max_k {
        return Err(EngineError::OutsideRange(record.id.clone(), temperature_k));
    }
    if let HeatModel::ReferenceOnly = record.model
        && temperature_k != record.reference_k
    {
        return Err(EngineError::ReferenceOnly(
            record.id.clone(),
            record.reference_k,
        ));
    }
    if let HeatModel::ConstantCp {
        joule_per_mol_kelvin: cp,
    } = record.model
        && (!cp.is_finite()
            || record.reference_k < record.min_k
            || record.reference_k > record.max_k)
    {
        return Err(EngineError::Invalid(format!(
            "Record '{}' has invalid Cp or a reference temperature outside the model range.",
            record.id
        )));
    }
    if let HeatModel::PolynomialCp { a, b, c, d } = record.model
        && (![a, b, c, d].into_iter().all(f64::is_finite)
            || record.reference_k < record.min_k
            || record.reference_k > record.max_k)
    {
        return Err(EngineError::Invalid(format!(
            "Record '{}' has invalid polynomial Cp coefficients or reference range.",
            record.id
        )));
    }
    if record
        .formation_enthalpy_j_mol
        .is_none_or(|value| !value.is_finite())
        || record
            .molar_entropy_j_mol_k
            .is_none_or(|value| !value.is_finite())
    {
        return Err(EngineError::MissingValues(record.id.clone()));
    }
    Ok(())
}

fn parse_formula(formula: &str) -> Result<BTreeMap<u8, u64>> {
    let chars: Vec<char> = formula
        .chars()
        .map(|c| match c {
            '₀' => '0',
            '₁' => '1',
            '₂' => '2',
            '₃' => '3',
            '₄' => '4',
            '₅' => '5',
            '₆' => '6',
            '₇' => '7',
            '₈' => '8',
            '₉' => '9',
            c => c,
        })
        .collect();
    if chars.is_empty() || chars.len() > 160 {
        return Err(EngineError::Invalid(format!(
            "Unsupported chemical formula '{formula}'."
        )));
    }
    let mut cursor = 0;
    parse_group(&chars, &mut cursor, None, 0).and_then(|counts| {
        if cursor == chars.len() {
            Ok(counts)
        } else {
            Err(EngineError::Invalid(format!(
                "Unsupported chemical formula '{formula}'."
            )))
        }
    })
}
fn parse_group(
    chars: &[char],
    cursor: &mut usize,
    close: Option<char>,
    depth: u8,
) -> Result<BTreeMap<u8, u64>> {
    if depth > 8 {
        return Err(EngineError::Invalid(
            "Chemical formula nesting is too deep.".into(),
        ));
    }
    let mut counts = BTreeMap::new();
    while *cursor < chars.len() {
        let c = chars[*cursor];
        if Some(c) == close {
            *cursor += 1;
            return if counts.is_empty() {
                Err(EngineError::Invalid("Empty chemical formula group.".into()))
            } else {
                Ok(counts)
            };
        }
        let group = if c == '(' || c == '[' {
            *cursor += 1;
            parse_group(
                chars,
                cursor,
                Some(if c == '(' { ')' } else { ']' }),
                depth + 1,
            )?
        } else if c.is_ascii_uppercase() {
            *cursor += 1;
            let mut symbol = c.to_string();
            if *cursor < chars.len() && chars[*cursor].is_ascii_lowercase() {
                symbol.push(chars[*cursor]);
                *cursor += 1;
            }
            let element = ELEMENTS
                .iter()
                .find(|element| element.symbol == symbol)
                .ok_or_else(|| {
                    EngineError::Invalid(format!("Unknown element '{symbol}' in formula."))
                })?;
            BTreeMap::from([(element.number, 1)])
        } else {
            return Err(EngineError::Invalid(format!(
                "Unsupported character '{c}' in chemical formula."
            )));
        };
        let multiplier = parse_count(chars, cursor)?;
        for (element, amount) in group {
            let value = amount
                .checked_mul(multiplier)
                .and_then(|value| {
                    counts
                        .get(&element)
                        .copied()
                        .unwrap_or(0u64)
                        .checked_add(value)
                })
                .ok_or(EngineError::Overflow)?;
            counts.insert(element, value);
        }
    }
    if close.is_some() {
        return Err(EngineError::Invalid(
            "Unclosed chemical formula group.".into(),
        ));
    }
    if counts.is_empty() {
        return Err(EngineError::Invalid("Empty chemical formula.".into()));
    }
    Ok(counts)
}
fn parse_count(chars: &[char], cursor: &mut usize) -> Result<u64> {
    let start = *cursor;
    let mut value = 0u64;
    while *cursor < chars.len() && chars[*cursor].is_ascii_digit() {
        value = value
            .checked_mul(10)
            .and_then(|value| value.checked_add(u64::from(chars[*cursor] as u8 - b'0')))
            .ok_or(EngineError::Overflow)?;
        *cursor += 1;
    }
    if start == *cursor {
        Ok(1)
    } else if value == 0 {
        Err(EngineError::Invalid(
            "Chemical formula subscripts must be positive.".into(),
        ))
    } else {
        Ok(value)
    }
}
