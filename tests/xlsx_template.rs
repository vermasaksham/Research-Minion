use crystal_forge::xlsx_template::{ExportRow, export_template};

#[test]
fn appends_results_to_copy_without_changing_template() {
    let folder = tempfile::tempdir().unwrap();
    let template_path = std::env::var("CRYSTALFORGE_TEMPLATE_TEST")
        .unwrap_or_else(|_| "tests/fixtures/thermo-template.xlsx".into());
    let template = std::path::Path::new(&template_path);
    let original = std::fs::read(template).unwrap();
    let output = std::env::var("CRYSTALFORGE_XLSX_TEST_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| folder.path().join("result.xlsx"));
    let rows = [ExportRow {
        temperature_k: 923.0,
        delta_h_j_mol: 100.0,
        delta_s_j_mol_k: 2.0,
        delta_g_j_mol: -1746.0,
        log10_k: 0.1,
        gas_pressure_bar: Some(1.2),
        log10_gas_pressure: Some(1.2_f64.log10()),
        electrode_potential_v: None,
        phases: "Sb(l); Cl2(g)".into(),
        provenance: "worksheet cell A1".into(),
    }];
    export_template(
        template,
        &output,
        "Example",
        "SbCl3 -> Sb + Cl2",
        "Synthetic test",
        &rows,
    )
    .unwrap();
    assert_eq!(std::fs::read(template).unwrap(), original);
    assert_ne!(std::fs::read(output).unwrap(), original);
    assert!(export_template(template, template, "Example", "Equation", "Source", &rows).is_err());
}
