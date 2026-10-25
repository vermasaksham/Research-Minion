use research_minion::{
    cvt::CvtConditions,
    model::EntryDraft,
    peak::{self, Analysis, OrthorhombicCell},
    store::Store,
};
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}

#[test]
fn bragg_matches_independent_sixty_degree_solution_and_inverse() {
    // λ=d=2 Å => sin θ=1/2 => 2θ=60° (analytic, independent reference).
    near(peak::two_theta_deg(2., 2.).unwrap().unwrap(), 60.);
    near(peak::d_spacing_a(2., 60.).unwrap(), 2.);
    near(peak::two_theta_deg(4., 2.).unwrap().unwrap(), 180.);
    near(peak::d_spacing_a(4., 180.).unwrap(), 2.);
    assert_eq!(peak::two_theta_deg(4.00001, 2.).unwrap(), None);
}

#[test]
fn orthorhombic_metric_matches_analytic_axes_and_mixed_indices() {
    let cell = OrthorhombicCell::new(3., 4., 12.).unwrap();
    near(cell.d_spacing([1, 0, 0]).unwrap(), 3.);
    near(cell.d_spacing([0, 2, 0]).unwrap(), 2.);
    near(cell.d_spacing([0, 0, 3]).unwrap(), 4.);
    // h²/a²+k²/b²+l²/c²=1/9+1/16+1/144=13/72.
    near(cell.d_spacing([1, 1, 1]).unwrap(), (72_f64 / 13.).sqrt());
    near(
        cell.d_spacing([-1, 1, -1]).unwrap(),
        cell.d_spacing([1, 1, 1]).unwrap(),
    );
    assert!(cell.d_spacing([0, 0, 0]).is_err());
}

#[test]
fn nonphysical_and_nonfinite_inputs_are_rejected_without_nan_results() {
    for invalid in [0., -1., f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(peak::two_theta_deg(invalid, 2.).is_err());
        assert!(peak::two_theta_deg(2., invalid).is_err());
        assert!(peak::d_spacing_a(invalid, 60.).is_err());
        assert!(OrthorhombicCell::new(invalid, 4., 5.).is_err());
    }
    for angle in [0., -1., 180.0001, f64::NAN, f64::INFINITY] {
        assert!(peak::d_spacing_a(1.5406, angle).is_err());
    }
    assert!(
        OrthorhombicCell {
            a: f64::NAN,
            b: 4.,
            c: 5.
        }
        .d_spacing([1, 0, 0])
        .is_err()
    );
    assert!(peak::two_theta_deg(f64::from_bits(1), f64::MAX).is_err());
}

#[test]
fn reflection_parser_is_explicit_signed_bounded_and_rejects_malformed_lines() {
    assert_eq!(
        peak::parse_reflections("1 0 0\n\n-1, 2, 3").unwrap(),
        vec![[1, 0, 0], [-1, 2, 3]]
    );
    for invalid in [
        "",
        "0 0 0",
        "1 0",
        "1 0 0 2",
        "1.5 0 0",
        "2147483648 0 0",
        "1 0 0\n1 0 0",
    ] {
        assert!(peak::parse_reflections(invalid).is_err(), "{invalid}");
    }
    let many = (1..=501)
        .map(|h| format!("{h} 0 0"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(peak::parse_reflections(&many).is_err());
}

#[test]
fn positions_retain_user_order_and_report_inaccessible_reflections() {
    let cell = OrthorhombicCell::new(2., 3., 4.).unwrap();
    let results = peak::calculate(cell, 2., &[[2, 0, 0], [0, 0, 1], [3, 0, 0]]).unwrap();
    assert_eq!(results[0].two_theta, Some(180.));
    assert_eq!(results[1].hkl, [0, 0, 1]);
    near(results[1].d_a, 4.);
    assert_eq!(results[2].two_theta, None);
}

#[test]
fn wavelength_and_axis_permutation_behave_consistently() {
    let cell = OrthorhombicCell::new(3., 4., 12.).unwrap();
    let permuted = OrthorhombicCell::new(4., 12., 3.).unwrap();
    near(
        cell.d_spacing([1, 2, 3]).unwrap(),
        permuted.d_spacing([2, 3, 1]).unwrap(),
    );
    let d = cell.d_spacing([1, 1, 1]).unwrap();
    assert!(
        peak::two_theta_deg(1.788965, d).unwrap().unwrap()
            > peak::two_theta_deg(1.5406, d).unwrap().unwrap()
    );
}

#[test]
fn analysis_snapshot_saves_exact_inputs_provenance_and_results_to_its_experiment() {
    let cell = OrthorhombicCell::new(3., 4., 12.).unwrap();
    let analysis = Analysis {
        cell,
        wavelength_a: 1.5406,
        results: peak::calculate(cell, 1.5406, &[[1, 1, 1]]).unwrap(),
        material: "User-entered trial".into(),
        setting: "axes as reported".into(),
        source: "Synthetic test cell, not a material reference".into(),
        interpretation: "Provisional\nRetain this interpretation".into(),
    };
    let report = analysis.report();
    for part in [
        "orthorhombic-bragg/1",
        "Wavelength = 1.5406 Å",
        "a = 3 Å; b = 4 Å; c = 12 Å",
        "1 1 1",
        "axes as reported",
        "Synthetic test cell",
        "no symmetry-absence",
        &analysis.interpretation,
    ] {
        assert!(report.contains(part), "{part}");
    }
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(folder.path()).unwrap();
    let experiment = store
        .create_cvt_experiment("Analysis target", CvtConditions::new(650., 600.).unwrap())
        .unwrap();
    let entry = store
        .create_entry(&EntryDraft {
            project_id: experiment.log_project_id.clone(),
            title: "XRD analysis".into(),
            body: report.clone(),
            event_date: chrono::NaiveDate::from_ymd_opt(2020, 2, 29).unwrap(),
        })
        .unwrap();
    drop(store);
    let reopened = Store::open(folder.path()).unwrap();
    assert_eq!(reopened.entry(&entry.id).unwrap().body, report);
    assert_eq!(
        reopened
            .entries(&experiment.log_project_id, "orthorhombic-bragg/1")
            .unwrap()
            .len(),
        1
    );
}
