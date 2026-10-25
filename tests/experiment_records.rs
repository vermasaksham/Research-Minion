use chrono::NaiveDate;
use research_minion::{
    cvt::CvtConditions,
    model::{EntryDraft, ExperimentDetails, HeatEffect, Material},
    store::{Store, StoreError},
};

fn details() -> ExperimentDetails {
    ExperimentDetails {
        experiment_date: Some(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()),
        heat_effect: HeatEffect::Endothermic,
        precursor: Material {
            text: "Sb + Se (weighed separately)".into(),
            elements: [34, 51].into(),
        },
        product: Material {
            text: "Sb₂Se₃ with possible surface oxide; interpretation provisional".into(),
            elements: [8, 34, 51].into(),
        },
        transport_agent: "I2 — recorded by the researcher".into(),
    }
}

#[test]
fn metadata_and_confirmed_elements_survive_reopen_without_reinterpreting_text() {
    let folder = tempfile::tempdir().unwrap();
    let original = {
        let store = Store::open(folder.path()).unwrap();
        store
            .create_cvt_experiment_with_details(
                "Materials trial",
                CvtConditions::new(500., 600.).unwrap(),
                &details(),
            )
            .unwrap()
    };
    let store = Store::open(folder.path()).unwrap();
    let reopened = store.cvt_experiment(&original.id).unwrap();
    assert_eq!(reopened, original);
    assert_eq!(reopened.details, details());
    assert_eq!(reopened.conditions.delta_c(), -100.);
    // Positive/negative ΔT never overwrites the researcher's thermal assignment.
    assert_eq!(reopened.details.heat_effect, HeatEffect::Endothermic);
    assert!(store.legacy_projects().unwrap().is_empty());
}

#[test]
fn optional_material_and_agent_text_does_not_require_element_selection() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(folder.path()).unwrap();
    let info = ExperimentDetails {
        precursor: Material {
            text: "Unknown proprietary mixture".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let saved = store
        .create_cvt_experiment_with_details(
            "Unassigned",
            CvtConditions::new(650., 600.).unwrap(),
            &info,
        )
        .unwrap();
    assert_eq!(store.cvt_experiment(&saved.id).unwrap().details, info);
    assert_eq!(saved.details.heat_effect, HeatEffect::Unspecified);
    assert!(saved.details.transport_agent.is_empty());
}

#[test]
fn each_experiment_has_an_independent_stable_log() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(folder.path()).unwrap();
    let a = store
        .create_cvt_experiment("A", CvtConditions::new(650., 600.).unwrap())
        .unwrap();
    let b = store
        .create_cvt_experiment("B", CvtConditions::new(700., 650.).unwrap())
        .unwrap();
    assert_ne!(a.log_project_id, b.log_project_id);
    let entry = store
        .create_entry(&EntryDraft {
            project_id: a.log_project_id.clone(),
            title: "Observation for A".into(),
            body: "Only linked to A".into(),
            event_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
        })
        .unwrap();
    assert_eq!(
        store.entries(&a.log_project_id, "").unwrap(),
        vec![entry.clone()]
    );
    assert!(store.entries(&b.log_project_id, "").unwrap().is_empty());
    drop(store);
    let reopened = Store::open(folder.path()).unwrap();
    assert_eq!(
        reopened.cvt_experiment(&a.id).unwrap().log_project_id,
        a.log_project_id
    );
    assert_eq!(
        reopened.entry(&entry.id).unwrap().project_id,
        a.log_project_id
    );
}

#[test]
fn invalid_elements_and_database_failures_never_create_partial_experiments_or_logs() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(folder.path()).unwrap();
    let mut invalid = details();
    invalid.precursor.elements.insert(0);
    assert!(matches!(
        store.create_cvt_experiment_with_details(
            "Invalid",
            CvtConditions::new(650., 600.).unwrap(),
            &invalid
        ),
        Err(StoreError::Validation(_))
    ));
    assert!(store.projects().unwrap().is_empty());
    let connection = rusqlite::Connection::open(store.path()).unwrap();
    connection.execute_batch("CREATE TRIGGER test_reject_element BEFORE INSERT ON experiment_elements BEGIN SELECT RAISE(ABORT,'simulated write failure'); END;").unwrap();
    assert!(
        store
            .create_cvt_experiment_with_details(
                "Failed save",
                CvtConditions::new(650., 600.).unwrap(),
                &details()
            )
            .is_err()
    );
    assert!(store.cvt_experiments().unwrap().is_empty());
    assert!(store.projects().unwrap().is_empty());
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM experiment_logs", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn schema_two_upgrade_preserves_experiments_and_unlinked_project_notes() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("research-minion.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(include_str!("fixtures/schema-v1.sql"))
        .unwrap();
    connection
        .execute_batch(include_str!("fixtures/schema-v2.sql"))
        .unwrap();
    connection.execute_batch("INSERT INTO projects VALUES ('prior-project','Prior notes','Do not reassign','2024-01-01T00:00:00Z'); INSERT INTO entries VALUES ('prior-entry','prior-project','Original','Keep these notes','2020-01-01','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z',1); INSERT INTO cvt_experiments VALUES ('old-a','Old A',650.5,600.25,'2024-01-02T00:00:00Z'); INSERT INTO cvt_experiments VALUES ('old-b','Old B',500,600,'2024-01-03T00:00:00Z');").unwrap();
    drop(connection);
    let store = Store::open(folder.path()).unwrap();
    let a = store.cvt_experiment("old-a").unwrap();
    let b = store.cvt_experiment("old-b").unwrap();
    assert_eq!(a.name, "Old A");
    assert_eq!(a.conditions.delta_c(), 50.25);
    assert_eq!(a.created_at.to_rfc3339(), "2024-01-02T00:00:00+00:00");
    assert_eq!(a.details, ExperimentDetails::default());
    assert_ne!(a.log_project_id, b.log_project_id);
    assert!(store.entries(&a.log_project_id, "").unwrap().is_empty());
    assert_eq!(
        store.entry("prior-entry").unwrap().project_id,
        "prior-project"
    );
    assert_eq!(store.legacy_projects().unwrap()[0].id, "prior-project");
    let link = a.log_project_id;
    drop(store);
    assert_eq!(
        Store::open(folder.path())
            .unwrap()
            .cvt_experiment("old-a")
            .unwrap()
            .log_project_id,
        link
    );
}
