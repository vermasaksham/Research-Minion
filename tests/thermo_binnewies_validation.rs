//! Independent reaction checks using selected rows of the user-supplied
//! Binnewies XLS table. The fixture is reproducible with
//! scripts/extract-binnewies-validation.py and is not Knacke-PDF verified.
use chrono::Utc;
use crystal_forge::thermodynamics::{
    HeatModel, PhaseMode, SpeciesRecord, calculate_phase_aware, parse_reaction,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct TableEntry {
    formula: String,
    phase: String,
    min_k: f64,
    max_k: f64,
    reference_k: f64,
    enthalpy_j_mol: f64,
    entropy_j_mol_k: f64,
    cp_coefficients: [f64; 4],
    source_locator: String,
}

fn table_records() -> Vec<SpeciesRecord> {
    let entries: Vec<TableEntry> =
        serde_json::from_str(include_str!("fixtures/binnewies-validation.json")).unwrap();
    assert_eq!(entries.len(), 9);
    entries
        .into_iter()
        .map(|entry| {
            let [a, b, c, d] = entry.cp_coefficients;
            assert!(
                entry
                    .source_locator
                    .starts_with("Binnewies.Tabelle1.XLS Tabelle1 row ")
            );
            SpeciesRecord {
                id: format!("table-{}-{}", entry.formula, entry.phase),
                source_id: "binnewies-xls-test".into(),
                source_name: "Binnewies XLS validation fixture".into(),
                source_authors: "User-supplied spreadsheet".into(),
                source_year: 2026,
                source_reference: "Binnewies.Tabelle1.XLS; original publication not checked".into(),
                source_enabled: true,
                source_locator: entry.source_locator,
                species_name: entry.formula.clone(),
                comments: "Test fixture only; no Knacke PDF verification".into(),
                imported_at: Utc::now(),
                updated_at: Utc::now(),
                revision: 1,
                verified_at: Some(Utc::now()),
                formula: entry.formula,
                phase: entry.phase,
                min_k: entry.min_k,
                max_k: entry.max_k,
                reference_k: entry.reference_k,
                reference_pressure_bar: 1.0,
                formation_enthalpy_j_mol: Some(entry.enthalpy_j_mol),
                molar_entropy_j_mol_k: Some(entry.entropy_j_mol_k),
                model: HeatModel::PolynomialCp { a, b, c, d },
            }
        })
        .collect()
}

#[test]
fn three_workbook_reactions_agree_with_independent_table_rows_to_source_precision() {
    let records = table_records();
    let cases = [
        (
            "SbCl3",
            "2/3 SbCl3(g) -> 2/3 Sb(s) + Cl2(g)",
            900.0,
            184_123.283_900_389_1,
            -10.686_028_518_448_671,
        ),
        (
            "SeCl2",
            "SeCl2(g) -> Se(s) + Cl2(g)",
            350.0,
            44_076.519_428_655_97,
            -6.577_931_920_208_868,
        ),
        (
            "TeCl2",
            "TeCl2(s) -> 1/2 Te2(g) + Cl2(g)",
            298.0,
            212_949.078_999_999_97,
            -37.325_843_280_235_27,
        ),
    ];
    for (name, equation, temperature, worksheet_g, worksheet_log_p) in cases {
        let reaction = parse_reaction(name, equation).unwrap();
        let result = calculate_phase_aware(
            &reaction,
            temperature,
            "binnewies-xls-test",
            &records,
            &PhaseMode::Explicit,
        )
        .unwrap();
        assert!(
            (result.delta_g_j_mol - worksheet_g).abs() < 20.0,
            "{name}: table ΔG={} vs worksheet {worksheet_g}",
            result.delta_g_j_mol
        );
        assert!(
            (result.log10_k - worksheet_log_p).abs() < 0.002,
            "{name}: table log10 K={} vs worksheet {worksheet_log_p}",
            result.log10_k
        );
    }
}

#[test]
fn bismuth_worksheet_does_not_agree_with_independent_table_rows() {
    let records = table_records();
    let reaction = parse_reaction("BiCl3", "BiCl3(g) -> BiCl(g) + Cl2(g)").unwrap();
    let result = calculate_phase_aware(
        &reaction,
        650.0,
        "binnewies-xls-test",
        &records,
        &PhaseMode::Explicit,
    )
    .unwrap();
    let worksheet_g = 205_086.329_515_771_18;
    let gap = result.delta_g_j_mol - worksheet_g;
    assert!(gap > 8_000.0 && gap < 8_350.0, "unexpected ΔG gap: {gap}");
}

#[test]
fn solid_antimony_is_outside_its_table_range_at_923_k() {
    let records = table_records();
    let reaction = parse_reaction("SbCl3", "2/3 SbCl3(g) -> 2/3 Sb(s) + Cl2(g)").unwrap();
    assert!(
        calculate_phase_aware(
            &reaction,
            923.0,
            "binnewies-xls-test",
            &records,
            &PhaseMode::Explicit,
        )
        .is_err()
    );
}
