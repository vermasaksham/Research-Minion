use crystal_forge::{
    cvt::{ABSOLUTE_ZERO_C, CvtConditions, TemperatureScale, temperature_text},
    store::{Store, StoreError},
};

#[test]
fn difference_keeps_the_source_minus_sink_convention() {
    for (source, sink, difference) in [
        (650.5, 600.25, 50.25),
        (500.0, 550.0, -50.0),
        (600.0, 600.0, 0.0),
        (-20.0, -30.0, 10.0),
    ] {
        let conditions = CvtConditions::new(source, sink).unwrap();
        assert_eq!(conditions.delta_c(), difference);
    }
}

#[test]
fn invalid_temperatures_are_rejected_in_either_field() {
    for value in [
        "",
        " ",
        "NaN",
        "inf",
        "-inf",
        "1e309",
        "650 °C",
        "warm",
        "-273.15001",
    ] {
        assert!(
            CvtConditions::parse(value, "600").is_err(),
            "source: {value}"
        );
        assert!(CvtConditions::parse("600", value).is_err(), "sink: {value}");
    }
    let pair = CvtConditions::parse(" -273.15 ", "0").unwrap();
    assert_eq!(pair.source_c(), ABSOLUTE_ZERO_C);
    assert_eq!(pair.delta_c(), ABSOLUTE_ZERO_C);
}

#[test]
fn experiments_save_without_a_project_and_reopen_with_exact_temperatures() {
    let folder = tempfile::tempdir().unwrap();
    let saved = {
        let store = Store::open(folder.path()).unwrap();
        assert!(store.projects().unwrap().is_empty());
        let conditions = CvtConditions::new(650.123456789, 599.987654321).unwrap();
        store
            .create_cvt_experiment("  CVT trial A  ", conditions)
            .unwrap()
    };
    let store = Store::open(folder.path()).unwrap();
    assert!(folder.path().join("crystal-forge.sqlite3").exists());
    let experiments = store.cvt_experiments().unwrap();
    assert_eq!(experiments, vec![saved.clone()]);
    assert_eq!(saved.name, "CVT trial A");
    assert_eq!(
        saved.conditions.source_c().to_bits(),
        650.123456789_f64.to_bits()
    );
    assert_eq!(
        saved.conditions.sink_c().to_bits(),
        599.987654321_f64.to_bits()
    );
    assert_eq!(saved.conditions.delta_c(), 650.123456789 - 599.987654321);
}

#[test]
fn unnamed_experiments_receive_stable_labels_and_all_records_remain_visible() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(folder.path()).unwrap();
    let first = store
        .create_cvt_experiment("", CvtConditions::new(500.0, 550.0).unwrap())
        .unwrap();
    let second = store
        .create_cvt_experiment(" ", CvtConditions::new(650.0, 650.0).unwrap())
        .unwrap();
    assert!(first.name.starts_with("CVT "));
    assert_ne!(first.id, second.id);
    assert_ne!(first.name, second.name);
    let records = store.cvt_experiments().unwrap();
    assert_eq!(records, vec![second, first]);
}

#[test]
fn version_one_migration_preserves_existing_research_records() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("research-minion.sqlite3");
    let original = rusqlite::Connection::open(&path).unwrap();
    original
        .execute_batch(include_str!("fixtures/schema-v1.sql"))
        .unwrap();
    original.execute("INSERT INTO projects VALUES ('old-project', 'Earlier research', 'Keep this project', '2024-01-01T12:00:00Z')", []).unwrap();
    original.execute("INSERT INTO entries VALUES ('old-entry', 'old-project', 'Original entry', 'Sb₂Se₃ notes: 550 °C', '2020-06-15', '2024-01-01T12:00:00Z', '2024-01-02T12:00:00Z', 3)", []).unwrap();
    drop(original);

    {
        let store = Store::open(folder.path()).unwrap();
        assert_eq!(store.projects().unwrap()[0].id, "old-project");
        let entry = store.entry("old-entry").unwrap();
        assert_eq!(entry.body, "Sb₂Se₃ notes: 550 °C");
        assert_eq!(entry.event_date.to_string(), "2020-06-15");
        assert_eq!(entry.created_at.to_rfc3339(), "2024-01-01T12:00:00+00:00");
        assert_eq!(entry.updated_at.to_rfc3339(), "2024-01-02T12:00:00+00:00");
        assert_eq!(entry.revision, 3);
        assert!(store.cvt_experiments().unwrap().is_empty());
        store
            .create_cvt_experiment("After upgrade", CvtConditions::new(650.0, 600.0).unwrap())
            .unwrap();
    }
    // Reopening must not repeat the migration or remove old/new records.
    let reopened = Store::open(folder.path()).unwrap();
    assert_eq!(reopened.path(), path.as_path());
    assert!(!folder.path().join("crystal-forge.sqlite3").exists());
    assert_eq!(reopened.cvt_experiments().unwrap().len(), 1);
    assert_eq!(reopened.entries("old-project", "").unwrap().len(), 1);
    let connection = rusqlite::Connection::open(&path).unwrap();
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 4);
}

#[test]
fn incompatible_version_one_database_is_not_partially_upgraded() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("crystal-forge.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TABLE unrelated (value TEXT); INSERT INTO unrelated VALUES ('keep me'); PRAGMA user_version = 1;").unwrap();
    drop(connection);
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        Store::open(folder.path()),
        Err(StoreError::Sqlite(_))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn overview_uses_a_common_scale_and_covers_every_set_point() {
    let pairs = [
        CvtConditions::new(650.0, 600.0).unwrap(),
        CvtConditions::new(500.0, 700.0).unwrap(),
    ];
    let scale = TemperatureScale::for_conditions(pairs);
    assert!(scale.min_c < 500.0 && scale.max_c > 700.0);
    assert!(scale.fraction(500.0) < scale.fraction(600.0));
    assert!(scale.fraction(600.0) < scale.fraction(650.0));
    assert!(scale.fraction(650.0) < scale.fraction(700.0));
    let ticks = scale.ticks();
    assert!(ticks.contains(&600.0));
    assert!(ticks.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn flat_negative_and_extreme_profiles_have_finite_plot_coordinates() {
    for pair in [
        CvtConditions::new(600.0, 600.0).unwrap(),
        CvtConditions::new(ABSOLUTE_ZERO_C, ABSOLUTE_ZERO_C).unwrap(),
        CvtConditions::new(f64::MAX, ABSOLUTE_ZERO_C).unwrap(),
        CvtConditions::new(f64::MAX, f64::MAX).unwrap(),
        CvtConditions::new(0.0, f64::from_bits(1)).unwrap(),
    ] {
        let scale = TemperatureScale::for_conditions([pair]);
        assert!(scale.min_c.is_finite() && scale.max_c.is_finite());
        assert!(scale.max_c > scale.min_c);
        for temperature in [pair.source_c(), pair.sink_c()] {
            let fraction = scale.fraction(temperature);
            assert!(fraction.is_finite() && (0.0..=1.0).contains(&fraction));
        }
        assert!(!scale.ticks().is_empty());
    }
}

#[test]
fn empty_overview_and_display_rounding_do_not_change_saved_conditions() {
    let scale = TemperatureScale::for_conditions([]);
    assert_eq!(scale.ticks(), vec![0.0, 200.0, 400.0, 600.0, 800.0, 1000.0]);
    assert_eq!(temperature_text(-0.0), "0");
    assert_eq!(temperature_text(650.25), "650.25");
    assert_eq!(temperature_text(0.3 - 0.2), "0.1");
    let pair = CvtConditions::new(650.123456789, 600.0).unwrap();
    assert_eq!(temperature_text(pair.source_c()), "650.123457");
    assert_eq!(pair.source_c(), 650.123456789);
}
