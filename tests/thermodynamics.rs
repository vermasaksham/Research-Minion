use chrono::Utc;
use crystal_forge::{
    thermo_database::{Phase, ThermoEntryDraft, ThermoStore},
    thermodynamics::{
        EngineError, GAS_CONSTANT_J_MOL_K, HeatModel, Rational, Reaction, ReactionTerm,
        SpeciesRecord, calculate,
    },
};

fn record(id: &str, formula: &str, h: f64, s: f64) -> SpeciesRecord {
    SpeciesRecord {
        id: id.into(),
        source_id: "synthetic".into(),
        source_name: "Synthetic test data".into(),
        source_authors: "CrystalForge tests".into(),
        source_year: 2026,
        source_reference: "No real publication".into(),
        source_enabled: true,
        source_locator: "test case".into(),
        species_name: format!("Synthetic {formula}"),
        comments: "Synthetic values only".into(),
        imported_at: Utc::now(),
        updated_at: Utc::now(),
        revision: 1,
        verified_at: Some(Utc::now()),
        formula: formula.into(),
        phase: "g".into(),
        min_k: 200.0,
        max_k: 1000.0,
        reference_k: 298.15,
        reference_pressure_bar: 1.0,
        formation_enthalpy_j_mol: Some(h),
        molar_entropy_j_mol_k: Some(s),
        model: HeatModel::ReferenceOnly,
    }
}
fn term(id: &str, num: i64, den: i64) -> ReactionTerm {
    ReactionTerm {
        species_id: id.into(),
        coefficient: Rational::new(num, den).unwrap(),
    }
}
fn hydrogen_chloride() -> (Reaction, Vec<SpeciesRecord>) {
    (
        Reaction {
            name: "Synthetic HCl formation".into(),
            terms: vec![term("h2", -1, 1), term("cl2", -1, 1), term("hcl", 2, 1)],
        },
        vec![
            record("h2", "H2", 0.0, 130.68),
            record("cl2", "Cl2", 0.0, 223.08),
            record("hcl", "HCl", -92_300.0, 186.69),
        ],
    )
}
fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual}, expected={expected}"
    );
}

#[test]
fn reference_temperature_reaction_matches_independent_arithmetic_and_keeps_provenance() {
    let (reaction, records) = hydrogen_chloride();
    let result = calculate(&reaction, 298.15, "synthetic", &records).unwrap();
    let expected_h = 2.0 * (-92_300.0);
    let expected_s = 2.0 * 186.69 - 130.68 - 223.08;
    let expected_g = expected_h - 298.15 * expected_s;
    close(result.delta_h_j_mol, expected_h, 1e-9);
    close(result.delta_s_j_mol_k, expected_s, 1e-9);
    close(result.delta_g_j_mol, expected_g, 1e-8);
    close(
        result.ln_k,
        -expected_g / (GAS_CONSTANT_J_MOL_K * 298.15),
        1e-12,
    );
    close(result.log10_k, result.ln_k / std::f64::consts::LN_10, 1e-12);
    close(
        result.equilibrium_constant.unwrap().ln(),
        result.ln_k,
        1e-12,
    );
    assert_eq!(result.standard_pressure_bar, 1.0);
    assert_eq!(result.species_used.len(), 3);
    assert_eq!(result.species_used[2].record.source_locator, "test case");
    assert_eq!(result.species_used[2].record.revision, 1);
}

#[test]
fn fractional_coefficients_and_grouped_formulas_balance_exactly() {
    let reaction = Reaction {
        name: "Fractional SbCl3 example".into(),
        terms: vec![term("sbcl3", -2, 3), term("sb", 2, 3), term("cl2", 1, 1)],
    };
    let records = vec![
        record("sbcl3", "SbCl3", -10.0, 1.0),
        record("sb", "Sb", 0.0, 1.0),
        record("cl2", "Cl₂", 0.0, 1.0),
    ];
    let result = calculate(&reaction, 298.15, "synthetic", &records).unwrap();
    close(result.delta_h_j_mol, 20.0 / 3.0, 1e-12);
    assert_eq!(Rational::new(2, 3).unwrap().numerator(), 2);
    assert_eq!(Rational::new(-4, -6).unwrap(), Rational::new(2, 3).unwrap());
    let grouped = Reaction {
        name: "Grouped formula test".into(),
        terms: vec![term("a", -1, 1), term("b", 1, 1), term("c", 3, 1)],
    };
    let grouped_records = vec![
        record("a", "Fe2(SO4)3", 0.0, 0.0),
        record("b", "Fe2O3", 0.0, 0.0),
        record("c", "SO3", 0.0, 0.0),
    ];
    assert!(calculate(&grouped, 298.15, "synthetic", &grouped_records).is_ok());
}

#[test]
fn a_temperature_model_requires_explicit_constant_cp_and_stays_inside_range() {
    let (reaction, mut records) = hydrogen_chloride();
    assert!(matches!(
        calculate(&reaction, 400.0, "synthetic", &records),
        Err(EngineError::ReferenceOnly(_, 298.15))
    ));
    for (record, cp) in records.iter_mut().zip([29.0, 33.0, 30.0]) {
        record.model = HeatModel::ConstantCp {
            joule_per_mol_kelvin: cp,
        };
    }
    let result = calculate(&reaction, 400.0, "synthetic", &records).unwrap();
    let reaction_cp = 2.0 * 30.0 - 29.0 - 33.0;
    close(
        result.delta_h_j_mol,
        -184_600.0 + reaction_cp * (400.0 - 298.15),
        1e-8,
    );
    close(
        result.delta_s_j_mol_k,
        19.62 + reaction_cp * (400.0_f64 / 298.15).ln(),
        1e-10,
    );
    assert!(matches!(
        calculate(&reaction, 1000.01, "synthetic", &records),
        Err(EngineError::OutsideRange(_, 1000.01))
    ));
    records[0].min_k = 300.0;
    assert!(matches!(
        calculate(&reaction, 400.0, "synthetic", &records),
        Err(EngineError::Invalid(_))
    ));
}

#[test]
fn unverified_missing_mixed_and_unbalanced_inputs_fail_closed() {
    let (reaction, mut records) = hydrogen_chloride();
    records[0].verified_at = None;
    assert!(matches!(
        calculate(&reaction, 298.15, "synthetic", &records),
        Err(EngineError::Unverified(_))
    ));
    records[0].verified_at = Some(Utc::now());
    records[1].source_id = "other".into();
    assert!(matches!(
        calculate(&reaction, 298.15, "synthetic", &records),
        Err(EngineError::SourceMismatch(_))
    ));
    records[1].source_id = "synthetic".into();
    records[1].reference_pressure_bar = 1.01325;
    assert_eq!(
        calculate(&reaction, 298.15, "synthetic", &records),
        Err(EngineError::MixedPressure)
    );
    records[1].reference_pressure_bar = 1.0;
    records[2].formation_enthalpy_j_mol = None;
    assert!(matches!(
        calculate(&reaction, 298.15, "synthetic", &records),
        Err(EngineError::MissingValues(_))
    ));
    records[2].formation_enthalpy_j_mol = Some(-92_300.0);
    records[2].formula = "HCl2".into();
    assert!(matches!(
        calculate(&reaction, 298.15, "synthetic", &records),
        Err(EngineError::Unbalanced(_))
    ));
    records[2].formula = "Unknown".into();
    assert!(matches!(
        calculate(&reaction, 298.15, "synthetic", &records),
        Err(EngineError::Invalid(_))
    ));
    assert!(matches!(
        calculate(&reaction, 0.0, "synthetic", &records),
        Err(EngineError::Invalid(_))
    ));
}

#[test]
fn existing_catalog_point_cp_never_becomes_an_implicit_temperature_model() {
    let folder = tempfile::tempdir().unwrap();
    let store = ThermoStore::open(folder.path()).unwrap();
    let source = store.sources().unwrap().remove(0);
    let mut ids = Vec::new();
    for (formula, h, s) in [
        ("H2", 0.0, 130.68),
        ("Cl2", 0.0, 223.08),
        ("HCl", -92_300.0, 186.69),
    ] {
        let draft = ThermoEntryDraft {
            source_id: source.id.clone(),
            species: formula.into(),
            formula: formula.into(),
            phase: Phase::Gas,
            min_k: 200.0,
            max_k: 1000.0,
            reference_k: 298.15,
            reference_pressure_bar: 1.0,
            enthalpy_j_mol: Some(h),
            entropy_j_mol_k: Some(s),
            heat_capacity_j_mol_k: Some(30.0),
            source_locator: "synthetic test only".into(),
            comments: "Not a Knacke value".into(),
        };
        let saved = store.create_entry(&draft).unwrap();
        ids.push(
            store
                .set_verified(&saved.id, saved.revision, true)
                .unwrap()
                .id,
        );
    }
    let records = store.engine_records(&source.id).unwrap();
    assert!(
        records
            .iter()
            .all(|record| record.model == HeatModel::ReferenceOnly)
    );
    let reaction = Reaction {
        name: "Synthetic values".into(),
        terms: vec![
            term(&ids[0], -1, 1),
            term(&ids[1], -1, 1),
            term(&ids[2], 2, 1),
        ],
    };
    assert!(calculate(&reaction, 298.15, &source.id, &records).is_ok());
    assert!(matches!(
        calculate(&reaction, 400.0, &source.id, &records),
        Err(EngineError::ReferenceOnly(_, _))
    ));
}

#[test]
fn k_overflow_or_underflow_keeps_logarithmic_result_without_false_zero_or_infinity() {
    let (reaction, mut records) = hydrogen_chloride();
    records[2].formation_enthalpy_j_mol = Some(-1e9);
    let large = calculate(&reaction, 298.15, "synthetic", &records).unwrap();
    assert!(large.log10_k.is_finite());
    assert!(large.equilibrium_constant.is_none());
    records[2].formation_enthalpy_j_mol = Some(1e9);
    let tiny = calculate(&reaction, 298.15, "synthetic", &records).unwrap();
    assert!(tiny.log10_k.is_finite());
    assert!(tiny.equilibrium_constant.is_none());
}
