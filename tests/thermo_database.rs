use crystal_forge::profiles::Profiles;
use crystal_forge::{
    thermo_database::{Phase, ThermoEntryDraft, ThermoError, ThermoStore},
    thermodynamics::HeatModel,
};

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
        model: HeatModel::ReferenceOnly,
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
fn worksheet_candidates_import_once_and_require_individual_verification() {
    let folder = tempfile::tempdir().unwrap();
    let db = ThermoStore::open(folder.path()).unwrap();
    let json = include_str!("../data/thermo/worksheet_candidates.json");
    assert_eq!(db.import_worksheet_candidates(json).unwrap(), 28);
    assert_eq!(db.import_worksheet_candidates(json).unwrap(), 0);
    let source = db
        .sources()
        .unwrap()
        .into_iter()
        .find(|s| s.name == "Worksheet: Red-SbCl3")
        .unwrap();
    let entries = db.entries(&source.id).unwrap();
    assert_eq!(entries.len(), 8);
    assert!(entries.iter().all(|entry| entry.verified_at.is_none()));
    assert!(entries.iter().any(|entry| entry.draft.formula == "Sb"
        && entry.draft.phase == Phase::Liquid
        && entry.draft.reference_k == 904.0));
    assert!(db.entries("knacke-1991").unwrap().is_empty());
}

#[test]
fn malformed_candidate_import_rolls_back_created_sources() {
    let folder = tempfile::tempdir().unwrap();
    let db = ThermoStore::open(folder.path()).unwrap();
    let mut candidates: serde_json::Value =
        serde_json::from_str(include_str!("../data/thermo/worksheet_candidates.json")).unwrap();
    candidates[0]["entries"][0]["phase"] = "invalid".into();
    assert!(
        db.import_worksheet_candidates(&candidates.to_string())
            .is_err()
    );
    assert_eq!(db.sources().unwrap().len(), 1);
}

#[test]
fn version_one_catalog_migrates_reference_records_without_changing_values() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("thermodynamics.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TABLE thermo_sources (id TEXT PRIMARY KEY,name TEXT,authors TEXT,year INTEGER,reference TEXT,enabled INTEGER,created_at TEXT);
        CREATE TABLE thermo_preferences (id INTEGER PRIMARY KEY,primary_source_id TEXT,fallback_source_id TEXT);
        CREATE TABLE thermo_entries (id TEXT PRIMARY KEY,source_id TEXT,species TEXT,formula TEXT,phase TEXT,min_k REAL,max_k REAL,reference_k REAL,model_type TEXT CHECK(model_type='reference_values'),enthalpy_j_mol REAL,entropy_j_mol_k REAL,heat_capacity_j_mol_k REAL,source_locator TEXT,comments TEXT,imported_at TEXT,updated_at TEXT,verified_at TEXT,revision INTEGER,reference_pressure_bar REAL);
        INSERT INTO thermo_sources VALUES ('sample','Sample','Tester',2026,'Test',1,'2026-01-01T00:00:00Z');
        INSERT INTO thermo_preferences VALUES (1,'sample',NULL);
        INSERT INTO thermo_entries VALUES ('old','sample','hydrogen','H2','g',200,1000,298.15,'reference_values',0,130.68,29,'page 1','old note','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z','2026-01-02T00:00:00Z',3,1);
        PRAGMA user_version=1;").unwrap();
    drop(connection);
    let db = ThermoStore::open_file(&path).unwrap();
    let old = db.entry("old").unwrap();
    assert_eq!(old.draft.formula, "H2");
    assert_eq!(old.draft.enthalpy_j_mol, Some(0.0));
    assert_eq!(old.draft.model, HeatModel::ReferenceOnly);
    assert!(old.verified_at.is_some());
    assert_eq!(old.revision, 3);
    assert_eq!(
        db.engine_records("sample").unwrap()[0].model,
        HeatModel::ReferenceOnly
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
