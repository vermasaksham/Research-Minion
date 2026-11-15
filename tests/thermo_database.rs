use crystal_forge::profiles::Profiles;
use crystal_forge::thermo_database::{Phase, ThermoEntryDraft, ThermoError, ThermoStore};

fn entry(source_id: &str) -> ThermoEntryDraft {
    ThermoEntryDraft {
        source_id: source_id.into(),
        species: "Example species".into(),
        formula: "XCl3".into(),
        phase: Phase::Gas,
        min_k: 300.0,
        max_k: 900.0,
        reference_k: 298.15,
        reference_pressure_bar: 1.0,
        enthalpy_j_mol: Some(-123_456.7),
        entropy_j_mol_k: Some(234.5),
        heat_capacity_j_mol_k: None,
        source_locator: "p. 42, table 3 (test data only)".into(),
        comments: "Synthetic values; not from Knacke".into(),
    }
}

#[test]
fn new_catalog_contains_knacke_metadata_but_no_unreviewed_values() {
    let folder = tempfile::tempdir().unwrap();
    let db = ThermoStore::open(folder.path()).unwrap();
    assert_eq!(db.path(), folder.path().join("thermodynamics.sqlite3"));
    assert_eq!(db.primary_source_id().unwrap(), "knacke-1991");
    let sources = db.sources().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].name, "Knacke 1991");
    assert!(db.entries("knacke-1991").unwrap().is_empty());
    drop(db);
    assert!(
        ThermoStore::open(folder.path())
            .unwrap()
            .entries("knacke-1991")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn phase_specific_entry_retains_units_provenance_and_verification_after_reopen() {
    let folder = tempfile::tempdir().unwrap();
    let db = ThermoStore::open(folder.path()).unwrap();
    let draft = entry("knacke-1991");
    let saved = db.create_entry(&draft).unwrap();
    assert_eq!(saved.draft, draft);
    assert!(saved.verified_at.is_none());
    let verified = db.set_verified(&saved.id, saved.revision, true).unwrap();
    assert!(verified.verified_at.is_some());
    assert_eq!(verified.revision, 2);
    drop(db);
    let reopened = ThermoStore::open(folder.path()).unwrap();
    assert_eq!(reopened.entry(&saved.id).unwrap(), verified);
    assert_eq!(reopened.entries("knacke-1991").unwrap(), vec![verified]);
}

#[test]
fn editing_clears_verification_and_stale_revisions_cannot_overwrite() {
    let folder = tempfile::tempdir().unwrap();
    let db = ThermoStore::open(folder.path()).unwrap();
    let saved = db.create_entry(&entry("knacke-1991")).unwrap();
    let verified = db.set_verified(&saved.id, saved.revision, true).unwrap();
    let mut revised = verified.draft.clone();
    revised.enthalpy_j_mol = Some(-120_000.0);
    let changed = db
        .update_entry(&saved.id, verified.revision, &revised)
        .unwrap();
    assert!(changed.verified_at.is_none());
    assert_eq!(changed.revision, 3);
    assert!(matches!(
        db.update_entry(&saved.id, verified.revision, &revised),
        Err(ThermoError::Conflict)
    ));
    assert!(matches!(
        db.set_verified(&saved.id, verified.revision, true),
        Err(ThermoError::Conflict)
    ));
}

#[test]
fn invalid_values_and_source_policy_cannot_create_partial_entries() {
    let folder = tempfile::tempdir().unwrap();
    let db = ThermoStore::open(folder.path()).unwrap();
    let mut draft = entry("knacke-1991");
    for invalid in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        draft.min_k = invalid;
        assert!(matches!(
            db.create_entry(&draft),
            Err(ThermoError::Validation(_))
        ));
    }
    draft.min_k = 1000.0;
    assert!(matches!(
        db.create_entry(&draft),
        Err(ThermoError::Validation(_))
    ));
    draft.min_k = 300.0;
    draft.enthalpy_j_mol = None;
    draft.entropy_j_mol_k = None;
    assert!(matches!(
        db.create_entry(&draft),
        Err(ThermoError::Validation(_))
    ));
    assert!(db.entries("knacke-1991").unwrap().is_empty());
    assert!(matches!(
        db.set_source_enabled("knacke-1991", false),
        Err(ThermoError::Validation(_))
    ));
    let custom = db
        .create_source("Lab data", "Research team", 2026, "Internal record")
        .unwrap();
    db.set_primary_source(&custom.id).unwrap();
    db.set_source_enabled("knacke-1991", false).unwrap();
    draft.source_id = "knacke-1991".into();
    draft.enthalpy_j_mol = Some(1.0);
    assert!(matches!(
        db.create_entry(&draft),
        Err(ThermoError::Validation(_))
    ));
    draft.source_id = custom.id.clone();
    let created = db.create_entry(&draft).unwrap();
    assert_eq!(created.draft.source_id, custom.id);
    assert!(matches!(
        db.delete_empty_source(&custom.id),
        Err(ThermoError::Validation(_))
    ));
}

#[test]
fn unrelated_and_newer_catalogs_are_not_replaced() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("thermodynamics.sqlite3");
    let sql = rusqlite::Connection::open(&file).unwrap();
    sql.execute_batch("CREATE TABLE unrelated (text TEXT); INSERT INTO unrelated VALUES ('keep');")
        .unwrap();
    drop(sql);
    assert!(matches!(
        ThermoStore::open(folder.path()),
        Err(ThermoError::UnrecognisedDatabase)
    ));
    let sql = rusqlite::Connection::open(&file).unwrap();
    sql.pragma_update(None, "user_version", 99).unwrap();
    drop(sql);
    assert!(matches!(
        ThermoStore::open(folder.path()),
        Err(ThermoError::UnsupportedVersion(99))
    ));
    let sql = rusqlite::Connection::open(&file).unwrap();
    assert_eq!(
        sql.query_row("SELECT text FROM unrelated", [], |row| row
            .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
}

#[test]
fn reference_catalog_is_global_while_research_workspaces_remain_profile_local() {
    let folder = tempfile::tempdir().unwrap();
    let mut profiles = Profiles::open(folder.path()).unwrap();
    let alice = profiles.create("Alice Example", None).unwrap();
    let bob = profiles.create("Bob Example", None).unwrap();
    let catalog = ThermoStore::open(profiles.data_root()).unwrap();
    let sample = catalog.create_entry(&entry("knacke-1991")).unwrap();
    assert_ne!(
        profiles.workspace(&alice.id).unwrap().path(),
        profiles.workspace(&bob.id).unwrap().path()
    );
    assert_eq!(catalog.path(), folder.path().join("thermodynamics.sqlite3"));
    drop(catalog);
    assert_eq!(
        ThermoStore::open(profiles.data_root())
            .unwrap()
            .entry(&sample.id)
            .unwrap(),
        sample
    );
}
