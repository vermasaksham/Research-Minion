//! Four independent Rust calculations against cached results in the supplied
//! chlorine workbook. These check arithmetic, not the original Knacke tables.
use chrono::Utc;
use crystal_forge::thermodynamics::{
    HeatModel, PhaseMode, SpeciesRecord, calculate_phase_aware, equilibrium_gas_pressure,
    parse_reaction, standard_electrode_potential,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Source {
    name: String,
    entries: Vec<Entry>,
}
#[derive(Deserialize)]
struct Entry {
    formula: String,
    phase: String,
    min_k: f64,
    max_k: f64,
    reference_k: f64,
    enthalpy_j_mol: f64,
    entropy_j_mol_k: f64,
    cp_coefficients: [f64; 4],
    source_locator: String,
    warning: Option<String>,
}

#[test]
fn four_worksheet_reactions_match_cached_excel_gibbs_pressure_and_potential() {
    let sources: Vec<Source> =
        serde_json::from_str(include_str!("../data/thermo/worksheet_candidates.json")).unwrap();
    let cases = [
        (
            "Red-SbCl3",
            "2/3 SbCl3(g) -> 2/3 Sb(s) + Cl2(g)",
            900.0,
            209_135.317_510_823_02,
            27.791_148_456_037_675,
            184_123.283_900_389_1,
            -10.686_028_518_448_671,
            -0.954_154_966_577_131_6,
        ),
        (
            "Red-SeCl2",
            "SeCl2(g) -> Se(s) + Cl2(g)",
            350.0,
            33_791.955_534_742_09,
            -29.384_468_268_325_385,
            44_076.519_428_655_97,
            -6.577_931_920_208_868,
            -0.228_411_252_674_799_03,
        ),
        (
            "Red-TeCl2",
            "TeCl2(s) -> 1/2 Te2(g) + Cl2(g)",
            400.0,
            267_256.406_640_422_8,
            183.692_192_929_836_86,
            193_779.529_468_488_1,
            -25.304_515_823_496_93,
            -1.004_195_105_293_507_2,
        ),
        (
            "Red-BiCl3",
            "BiCl3(g) -> BiCl(g) + Cl2(g)",
            650.0,
            314_340.080_558_397_5,
            168.082_693_911_732_75,
            205_086.329_515_771_18,
            -16.480_617_581_304_212,
            -1.062_788_669_304_923_8,
        ),
    ];
    for (
        name,
        equation,
        temperature,
        expected_h,
        expected_s,
        expected_g,
        expected_log_p,
        expected_e,
    ) in cases
    {
        let source = sources.iter().find(|source| source.name == name).unwrap();
        if name == "Red-BiCl3" {
            assert!(
                source
                    .entries
                    .iter()
                    .find(|entry| entry.formula == "BiCl3" && entry.phase == "g")
                    .and_then(|entry| entry.warning.as_deref())
                    .unwrap()
                    .contains("DISCREPANCY")
            );
        }
        let records: Vec<_> = source
            .entries
            .iter()
            .map(|entry| {
                let [a, b, c, d] = entry.cp_coefficients;
                SpeciesRecord {
                    id: format!("{}-{}-{}", name, entry.formula, entry.phase),
                    source_id: name.into(),
                    source_name: name.into(),
                    source_authors: "User worksheet".into(),
                    source_year: 2026,
                    source_reference: "Partial_pressure_Chlorine_Sb-Bi-Te-Se.xlsx".into(),
                    source_enabled: true,
                    source_locator: entry.source_locator.clone(),
                    species_name: entry.formula.clone(),
                    comments: "Validation fixture, not Knacke-verified".into(),
                    imported_at: Utc::now(),
                    updated_at: Utc::now(),
                    revision: 1,
                    verified_at: Some(Utc::now()),
                    formula: entry.formula.clone(),
                    phase: entry.phase.clone(),
                    min_k: entry.min_k,
                    max_k: entry.max_k,
                    reference_k: entry.reference_k,
                    reference_pressure_bar: 1.0,
                    formation_enthalpy_j_mol: Some(entry.enthalpy_j_mol),
                    molar_entropy_j_mol_k: Some(entry.entropy_j_mol_k),
                    model: HeatModel::PolynomialCp { a, b, c, d },
                }
            })
            .collect();
        let reaction = parse_reaction(name, equation).unwrap();
        let result =
            calculate_phase_aware(&reaction, temperature, name, &records, &PhaseMode::Explicit)
                .unwrap();
        assert!(
            (result.delta_h_j_mol - expected_h).abs() < 0.05,
            "{name}: ΔH mismatch"
        );
        assert!(
            (result.delta_s_j_mol_k - expected_s).abs() < 0.000_1,
            "{name}: ΔS mismatch"
        );
        assert!(
            (result.delta_g_j_mol - expected_g).abs() < 0.05,
            "{name}: ΔG {} vs {expected_g}",
            result.delta_g_j_mol
        );
        assert!(
            (result.log10_k - expected_log_p).abs() < 0.000_01,
            "{name}: logK {} vs {expected_log_p}",
            result.log10_k
        );
        let target = result
            .species_used
            .iter()
            .find(|used| used.record.formula == "Cl2" && used.record.phase == "g")
            .unwrap();
        let activities = result
            .species_used
            .iter()
            .filter(|used| used.record.phase == "g" && used.record.id != target.record.id)
            .map(|used| (used.record.id.clone(), 1.0))
            .collect();
        let (relative, _bar) =
            equilibrium_gas_pressure(&result, &target.record.id, &activities).unwrap();
        assert!((relative.log10() - expected_log_p).abs() < 0.000_01);
        assert!(
            (standard_electrode_potential(&result, 2).unwrap() - expected_e).abs() < 0.000_01,
            "{name}: E° mismatch"
        );
        assert!(
            (result.delta_h_j_mol - temperature * result.delta_s_j_mol_k - result.delta_g_j_mol)
                .abs()
                < 1e-7
        );
    }
}
