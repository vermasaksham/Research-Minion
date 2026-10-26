use research_minion::{
    cvt::CvtConditions,
    model::EntryDraft,
    peak_reference::{self as r, CrystalCell, ExplorerParameters, ReferenceSnapshot},
    profiles::Profiles,
};
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}

#[test]
fn every_emission_value_uncertainty_and_edge_matches_the_retained_nist_direct_columns() {
    let c = r::catalog().unwrap();
    assert_eq!(c.edx.len(), 46);
    assert_eq!(c.elements.len(), 7);
    for line in &c.edx {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("data/peak")
                .join(&line.source_file),
        )
        .unwrap();
        let rows: Vec<_> = text
            .lines()
            .skip(5)
            .map(|row| row.split('\t').collect::<Vec<_>>())
            .collect();
        for transition in &line.source_transitions {
            let row = rows.iter().find(|row| row[2] == transition).unwrap();
            near(row[5].parse().unwrap(), line.energy_ev);
            near(row[6].parse().unwrap(), line.uncertainty_ev);
            assert_eq!(row[5], line.reported_energy_ev);
            assert_eq!(row[6], line.reported_uncertainty_ev);
        }
        let edge_label = format!("{} edge", line.shell);
        let edge = rows.iter().find(|row| row[2] == edge_label).unwrap();
        near(edge[5].parse().unwrap(), line.edge_ev);
        near(edge[6].parse().unwrap(), line.edge_uncertainty_ev);
        near(line.energy_kev() * 1000., line.energy_ev);
    }
}
#[test]
fn blends_are_single_markers_and_missing_direct_measurements_are_not_invented() {
    let c = r::catalog().unwrap();
    for (element, transition) in [
        ("Se", "L3M4,5"),
        ("Br", "L3M4,5"),
        ("Cl", "KM"),
        ("Sb", "L3N4,5"),
        ("I", "L3N4,5"),
    ] {
        let lines: Vec<_> = c
            .edx
            .iter()
            .filter(|l| l.element == element && l.transition == transition)
            .collect();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].blend);
        assert_eq!(lines[0].source_transitions.len(), 2);
    }
    assert!(
        !c.edx
            .iter()
            .any(|l| l.element == "S" && l.transition == "KM2")
    );
    assert_eq!(
        c.edx
            .iter()
            .find(|l| l.id == "Sb-L3M5")
            .unwrap()
            .energy_text_kev(),
        "3.604756"
    );
    assert_eq!(
        c.edx.iter().find(|l| l.id == "Bi-KL3").unwrap().mass_number,
        "209"
    );
}
#[test]
fn general_cell_metric_matches_analytic_orthorhombic_hexagonal_and_rhombohedral_cases() {
    let ortho = CrystalCell {
        lengths_a: [3., 4., 12.],
        angles_deg: [90.; 3],
    };
    near(ortho.d_spacing([1, 1, 1]).unwrap(), (72_f64 / 13.).sqrt());
    near(
        ortho.d_spacing([-1, 1, -1]).unwrap(),
        ortho.d_spacing([1, 1, 1]).unwrap(),
    );
    let hex = CrystalCell {
        lengths_a: [3., 3., 5.],
        angles_deg: [90., 90., 120.],
    };
    near(hex.d_spacing([1, 0, 0]).unwrap(), 3. * 3_f64.sqrt() / 2.);
    near(hex.d_spacing([0, 0, 2]).unwrap(), 2.5);
    let rhombo = CrystalCell {
        lengths_a: [2.; 3],
        angles_deg: [60.; 3],
    };
    near(rhombo.d_spacing([1, 0, 0]).unwrap(), (8_f64 / 3.).sqrt());
}
#[test]
fn all_bundled_reflections_match_independent_gemmi_metric_values() {
    let c = r::catalog().unwrap();
    let mut count = 0;
    for p in &c.phases {
        for row in &p.reflections {
            near(p.cell().d_spacing(row.hkl).unwrap(), row.d_a);
            count += 1;
        }
    }
    assert!(count >= 48);
}
#[test]
fn pbnm_to_pnma_maps_both_cell_and_indices_without_moving_reflections() {
    let c = r::catalog().unwrap();
    let p = c.phases.iter().find(|p| p.id == "sb2se3").unwrap();
    let first = r::diffraction_positions(c, &["sb2se3".into()], 1.5406, false).unwrap();
    let mapped = r::diffraction_positions(c, &["sb2se3".into()], 1.5406, true).unwrap();
    assert_eq!(r::pbnm_to_pnma([2, 4, 0]), [4, 0, 2]);
    assert_eq!(p.cell().cycle_axes().lengths_a, [11.77, 3.962, 11.62]);
    for row in first {
        let other = mapped.iter().find(|r| r.id == row.id).unwrap();
        near(row.d_a, other.d_a);
        near(row.two_theta_deg.unwrap(), other.two_theta_deg.unwrap());
        assert_eq!(other.displayed_hkl, r::pbnm_to_pnma(row.source_hkl));
    }
}
#[test]
fn changing_diffraction_radiation_moves_angles_but_never_emission_energies() {
    let c = r::catalog().unwrap();
    let a = r::snapshot(c, ExplorerParameters::default(), "").unwrap();
    let p = ExplorerParameters {
        wavelength_a: c
            .radiation
            .iter()
            .find(|r| r.id == "co")
            .unwrap()
            .wavelength_a,
        ..ExplorerParameters::default()
    };
    let b = r::snapshot(c, p, "").unwrap();
    for (x, y) in a.edx.markers.iter().zip(&b.edx.markers) {
        assert_eq!(x.energy_ev, y.energy_ev);
    }
    let x = a
        .xrd
        .positions
        .iter()
        .find(|r| r.id == "sb2se3-240")
        .unwrap();
    let y = b.xrd.positions.iter().find(|r| r.id == x.id).unwrap();
    near(x.d_a, y.d_a);
    assert!(y.two_theta_deg.unwrap() > x.two_theta_deg.unwrap());
}
#[test]
fn beam_threshold_uses_the_ionisation_edge_not_the_emitted_line_energy() {
    let c = r::catalog().unwrap();
    let line = c.edx.iter().find(|l| l.id == "Se-KL3").unwrap();
    assert!(12000. > line.energy_ev);
    assert!(!r::above_shell_edge(12., line.edge_ev).unwrap());
    assert!(!r::above_shell_edge(1., 1000.).unwrap());
    assert!(r::above_shell_edge(1.001, 1000.).unwrap());
    assert!(r::above_shell_edge(95., 90537.7).unwrap());
}
#[test]
fn photon_conversion_agrees_with_exact_si_constants_and_is_not_an_angle() {
    let hc = 6.626_070_15e-34 * 299_792_458. / 1.602_176_634e-19 * 1e10;
    near(r::HC_EV_A, hc);
    near(
        r::photon_wavelength_a(8047.8227).unwrap(),
        1.5405930654163176,
    );
    for invalid in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(r::photon_wavelength_a(invalid).is_err());
        assert!(r::above_shell_edge(invalid, 1000.).is_err());
    }
}
#[test]
fn invalid_cells_selections_and_numeric_ranges_are_rejected() {
    let c = r::catalog().unwrap();
    for angles in [
        [0., 90., 90.],
        [180., 90., 90.],
        [20., 20., 170.],
        [f64::NAN, 90., 90.],
    ] {
        assert!(
            CrystalCell {
                lengths_a: [3.; 3],
                angles_deg: angles
            }
            .d_spacing([1, 0, 0])
            .is_err()
        );
    }
    assert!(
        CrystalCell {
            lengths_a: [3.; 3],
            angles_deg: [90.; 3]
        }
        .d_spacing([0; 3])
        .is_err()
    );
    for invalid in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(r::diffraction_positions(c, &["sb2se3".into()], invalid, false).is_err());
    }
    assert!(r::diffraction_positions(c, &["unknown".into()], 1., false).is_err());
    let mut p = ExplorerParameters::default();
    p.elements.push("Un".into());
    assert!(r::snapshot(c, p, "").is_err());
    let p = ExplorerParameters {
        family: Some("fictional".into()),
        ..ExplorerParameters::default()
    };
    assert!(r::snapshot(c, p, "").is_err());
    let rows = r::diffraction_positions(c, &["sb2se3".into()], 100., false).unwrap();
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|r| r.two_theta_deg.is_none()));
}
#[test]
fn empty_selections_and_line_families_do_not_create_phantom_peaks() {
    let c = r::catalog().unwrap();
    let mut p = ExplorerParameters::default();
    p.elements.clear();
    p.phases.clear();
    let s = r::snapshot(c, p.clone(), "").unwrap();
    assert!(s.edx.markers.is_empty());
    assert!(s.xrd.positions.is_empty());
    p.elements = vec!["S".into()];
    p.family = Some("L".into());
    assert!(r::snapshot(c, p, "").unwrap().edx.markers.is_empty());
    let p = ExplorerParameters {
        energy_max_kev: 5.,
        ..ExplorerParameters::default()
    };
    let s = r::snapshot(c, p, "").unwrap();
    assert!(s.edx.markers.iter().any(|l| l.energy_kev() > 5.));
}
#[test]
fn snapshot_and_markdown_report_retain_units_sources_settings_and_interpretation() {
    let s = r::snapshot(
        r::catalog().unwrap(),
        ExplorerParameters::default(),
        "Sb + Se is not proof of Sb₂Se₃.",
    )
    .unwrap();
    let json = s.json().unwrap();
    let decoded: ReferenceSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.edx.energy_unit, "eV");
    assert_eq!(decoded.xrd.angle_unit, "degree_2theta");
    assert_eq!(decoded.xrd.length_unit, "angstrom");
    assert_eq!(decoded.interpretation, s.interpretation);
    assert_eq!(decoded.xrd.phases[0].cif_text, s.xrd.phases[0].cif_text);
    let report = s.report().unwrap();
    for text in [
        "NIST",
        "COD 9007437",
        "3604.756",
        "degree_2theta",
        "uncertainty",
        "sha256",
        &s.interpretation,
    ] {
        assert!(report.contains(text), "{text}");
    }
}
#[test]
fn reference_snapshots_and_exports_remain_inside_the_owning_profile_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let mut registry = Profiles::open(dir.path()).unwrap();
    let owner = registry.create("Researcher A", None).unwrap();
    let other = registry.create("Researcher B", None).unwrap();
    let store = registry.workspace(&owner.id).unwrap();
    let cvt = store
        .create_cvt_experiment("Reference trial", CvtConditions::new(650., 600.).unwrap())
        .unwrap();
    let s = r::snapshot(
        r::catalog().unwrap(),
        ExplorerParameters::default(),
        "Private interpretation 🧪",
    )
    .unwrap();
    let body = s.report().unwrap();
    let entry = store
        .create_entry(&EntryDraft {
            project_id: cvt.log_project_id.clone(),
            title: "Peak reference".into(),
            body: body.clone(),
            event_date: chrono::NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
        })
        .unwrap();
    let path = r::export_to_profile(&s, store.path().parent().unwrap()).unwrap();
    let another = r::export_to_profile(&s, store.path().parent().unwrap()).unwrap();
    assert_ne!(path, another);
    assert!(path.starts_with(dir.path().join("profiles").join(&owner.id)));
    assert!(!path.starts_with(dir.path().join("profiles").join(&other.id)));
    assert_eq!(std::fs::read_to_string(path).unwrap(), s.json().unwrap());
    drop(store);
    let other_store = registry.workspace(&other.id).unwrap();
    assert!(other_store.entry(&entry.id).is_err());
    let reopened = registry.workspace(&owner.id).unwrap();
    assert_eq!(reopened.entry(&entry.id).unwrap().body, body);
}
#[test]
fn tampered_metric_or_experimental_value_fails_catalog_validation() {
    let mut c = r::catalog().unwrap().clone();
    c.phases[0].reflections[0].d_a += 0.01;
    assert!(c.validate().is_err());
    let mut c = r::catalog().unwrap().clone();
    c.edx[0].energy_ev += 1.;
    assert!(c.validate().is_err());
    let mut c = r::catalog().unwrap().clone();
    c.schema = 99;
    assert!(c.validate().is_err());
}
