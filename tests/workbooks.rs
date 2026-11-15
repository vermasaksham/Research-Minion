use crystal_forge::{
    cvt::CvtConditions,
    model::{ExperimentDetails, MassUnit, StartingMaterial},
    store::{Store, StoreError},
};

#[test]
fn version_four_experiments_and_notes_migrate_into_one_default_workbook() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("crystal-forge.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(include_str!("fixtures/schema-v3.sql"))
        .unwrap();
    connection.execute_batch("ALTER TABLE cvt_experiments ADD COLUMN experiment_date TEXT;
        ALTER TABLE cvt_experiments ADD COLUMN deleted_at TEXT;
        ALTER TABLE entries ADD COLUMN deleted_at TEXT;
        CREATE INDEX entries_deleted ON entries(project_id, deleted_at);
        PRAGMA user_version=4;
        INSERT INTO projects VALUES ('old-log','Experiment log','Keep','2024-01-01T00:00:00Z');
        INSERT INTO projects VALUES ('previous','Earlier notes','Keep','2024-01-01T00:00:00Z');
        INSERT INTO cvt_experiments (id,name,source_c,sink_c,created_at,heat_effect,precursor_text,product_text,transport_agent,experiment_date,deleted_at)
            VALUES ('old-exp','Old experiment',650,600,'2024-01-01T00:00:00Z','unspecified','Sb + Se','Sb2Se3','I2','2023-12-31',NULL);
        INSERT INTO experiment_logs VALUES ('old-exp','old-log');
        INSERT INTO entries (id,project_id,title,body,event_date,created_at,updated_at,revision,deleted_at)
            VALUES ('note','old-log','Observation','Original note','2024-01-01','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z',1,NULL);").unwrap();
    drop(connection);
    let store = Store::open(folder.path()).unwrap();
    let workbooks = store.workbooks().unwrap();
    assert_eq!(workbooks.len(), 1);
    assert_eq!(store.active_workbook_id().unwrap(), workbooks[0].id);
    let experiment = store.cvt_experiment("old-exp").unwrap();
    assert_eq!(experiment.workbook_id, workbooks[0].id);
    assert!(experiment.details.starting_materials.is_empty());
    assert_eq!(store.entry("note").unwrap().body, "Original note");
    assert_eq!(store.legacy_projects().unwrap()[0].id, "previous");
    drop(store);
    let reopened = Store::open(folder.path()).unwrap();
    assert_eq!(reopened.workbooks().unwrap(), workbooks);
    assert_eq!(
        reopened.cvt_experiment("old-exp").unwrap().log_project_id,
        "old-log"
    );
}

#[test]
fn workbooks_switch_rename_and_move_without_changing_experiment_or_note_identity() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(folder.path()).unwrap();
    let original_workbook = store.active_workbook_id().unwrap();
    let first = store
        .create_cvt_experiment("First", CvtConditions::new(650., 600.).unwrap())
        .unwrap();
    let second_workbook = store
        .create_workbook("Sb2Se3 Bulk CVT", "Paper 1", "Sb-Se", "Sb2Se3")
        .unwrap();
    store.set_active_workbook(&second_workbook.id).unwrap();
    let second = store
        .create_cvt_experiment("Second", CvtConditions::new(700., 650.).unwrap())
        .unwrap();
    assert_eq!(first.workbook_id, original_workbook);
    assert_eq!(second.workbook_id, second_workbook.id);
    let explicit = store
        .create_cvt_experiment_in_workbook(
            "Explicit first",
            CvtConditions::new(450., 400.).unwrap(),
            &ExperimentDetails::default(),
            &original_workbook,
        )
        .unwrap();
    assert_eq!(explicit.workbook_id, original_workbook);
    store
        .rename_workbook(&second_workbook.id, "Sb2Se3 Iodine Series")
        .unwrap();
    store
        .move_experiment_to_workbook(&first.id, &second_workbook.id)
        .unwrap();
    let moved = store.cvt_experiment(&first.id).unwrap();
    assert_eq!(moved.workbook_id, second_workbook.id);
    assert_eq!(moved.log_project_id, first.log_project_id);
    assert_eq!(moved.conditions, first.conditions);
    assert!(matches!(
        store.move_experiment_to_workbook(&first.id, "missing"),
        Err(StoreError::NotFound)
    ));
    drop(store);
    let reopened = Store::open(folder.path()).unwrap();
    assert_eq!(reopened.active_workbook_id().unwrap(), second_workbook.id);
    assert_eq!(reopened.cvt_experiment(&first.id).unwrap(), moved);
    assert_eq!(
        reopened
            .workbooks()
            .unwrap()
            .iter()
            .find(|w| w.id == second_workbook.id)
            .unwrap()
            .name,
        "Sb2Se3 Iodine Series"
    );
}

#[test]
fn starting_material_masses_keep_units_and_invalid_rows_cannot_save() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(folder.path()).unwrap();
    let mut details = ExperimentDetails {
        starting_materials: vec![
            StartingMaterial {
                name: "Sb".into(),
                mass: "0.2500".into(),
                unit: MassUnit::G,
            },
            StartingMaterial {
                name: "Se".into(),
                mass: "120".into(),
                unit: MassUnit::Mg,
            },
        ],
        ..Default::default()
    };
    let saved = store
        .create_cvt_experiment_with_details(
            "Mass trial",
            CvtConditions::new(500., 450.).unwrap(),
            &details,
        )
        .unwrap();
    assert_eq!(
        store
            .cvt_experiment(&saved.id)
            .unwrap()
            .details
            .starting_materials,
        details.starting_materials
    );
    details.starting_materials[0].mass = "-2".into();
    assert!(matches!(
        store.create_cvt_experiment_with_details(
            "Bad",
            CvtConditions::new(500., 450.).unwrap(),
            &details
        ),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(store.cvt_experiments().unwrap().len(), 1);
}
