use chrono::NaiveDate;
use crystal_forge::{
    cvt::CvtConditions,
    editor::Editor,
    model::{EntryDraft, ExperimentDetails},
    store::{Store, StoreError},
};

fn note(project_id: &str) -> EntryDraft {
    EntryDraft {
        project_id: project_id.into(),
        title: "Original observation".into(),
        body: "Exact notes\nSb₂Se₃ 650 °C".into(),
        event_date: NaiveDate::from_ymd_opt(2020, 2, 29).unwrap(),
    }
}

#[test]
fn deleted_experiment_restores_its_original_metadata_and_log_after_restart() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(folder.path()).unwrap();
    let details = ExperimentDetails {
        experiment_date: Some(NaiveDate::from_ymd_opt(2020, 2, 29).unwrap()),
        ..Default::default()
    };
    let original = store
        .create_cvt_experiment_with_details(
            "Retrospective CVT",
            CvtConditions::new(650., 600.).unwrap(),
            &details,
        )
        .unwrap();
    let entry = store.create_entry(&note(&original.log_project_id)).unwrap();
    let other = store
        .create_cvt_experiment("Unaffected", CvtConditions::new(700., 650.).unwrap())
        .unwrap();
    store.delete_experiment(&original.id).unwrap();
    assert_eq!(store.cvt_experiments().unwrap(), vec![other]);
    assert!(matches!(
        store.cvt_experiment(&original.id),
        Err(StoreError::NotFound)
    ));
    assert!(matches!(store.entry(&entry.id), Err(StoreError::NotFound)));
    assert!(
        store
            .entries(&original.log_project_id, "")
            .unwrap()
            .is_empty()
    );
    assert!(store.create_entry(&note(&original.log_project_id)).is_err());
    assert!(
        store
            .update_entry(&entry.id, entry.revision, &note(&original.log_project_id))
            .is_err()
    );
    assert!(store.legacy_projects().unwrap().is_empty());
    drop(store);
    let reopened = Store::open(folder.path()).unwrap();
    assert_eq!(
        reopened.deleted_experiments().unwrap(),
        vec![original.clone()]
    );
    reopened.restore_experiment(&original.id).unwrap();
    assert_eq!(reopened.cvt_experiment(&original.id).unwrap(), original);
    assert_eq!(reopened.entry(&entry.id).unwrap(), entry);
    assert!(reopened.deleted_experiments().unwrap().is_empty());
}

#[test]
fn entry_deletion_restoration_preserves_content_and_invalidates_stale_editors() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(folder.path()).unwrap();
    let project = store.create_project("Log", "").unwrap();
    let original = store.create_entry(&note(&project.id)).unwrap();
    let mut stale = Editor::from_entry(original.clone());
    stale.body = "Unsaved stale interpretation".into();
    store.delete_entry(&original.id, original.revision).unwrap();
    assert!(store.entries(&project.id, "Sb₂Se₃").unwrap().is_empty());
    assert!(matches!(stale.save(&store), Err(StoreError::NotFound)));
    assert_eq!(stale.body, "Unsaved stale interpretation");
    drop(store);
    let store = Store::open(folder.path()).unwrap();
    let deleted = store.deleted_entries().unwrap().pop().unwrap();
    assert_eq!(deleted.body, original.body);
    assert_eq!(deleted.created_at, original.created_at);
    assert_eq!(deleted.revision, 2);
    assert!(store.restore_entry(&deleted.id, 1).is_err());
    store.restore_entry(&deleted.id, deleted.revision).unwrap();
    let restored = store.entry(&original.id).unwrap();
    assert_eq!(restored.body, original.body);
    assert_eq!(restored.event_date, original.event_date);
    assert_eq!(restored.created_at, original.created_at);
    assert_eq!(restored.updated_at, original.updated_at);
    assert_eq!(restored.revision, 3);
    assert!(matches!(stale.save(&store), Err(StoreError::Conflict)));
}

#[test]
fn stale_deletion_cannot_remove_a_concurrently_updated_note() {
    let folder = tempfile::tempdir().unwrap();
    let first = Store::open(folder.path()).unwrap();
    let project = first.create_project("Log", "").unwrap();
    let original = first.create_entry(&note(&project.id)).unwrap();
    let second = Store::open(folder.path()).unwrap();
    let updated = second
        .update_entry(
            &original.id,
            1,
            &EntryDraft {
                body: "New observation".into(),
                ..note(&project.id)
            },
        )
        .unwrap();
    assert!(matches!(
        first.delete_entry(&original.id, 1),
        Err(StoreError::Conflict)
    ));
    assert_eq!(first.entry(&original.id).unwrap(), updated);
    assert!(first.deleted_entries().unwrap().is_empty());
}

#[test]
fn individually_deleted_note_stays_deleted_when_parent_experiment_restores() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(folder.path()).unwrap();
    let experiment = store
        .create_cvt_experiment("CVT", CvtConditions::new(650., 600.).unwrap())
        .unwrap();
    let entry = store
        .create_entry(&note(&experiment.log_project_id))
        .unwrap();
    store.delete_entry(&entry.id, 1).unwrap();
    store.delete_experiment(&experiment.id).unwrap();
    assert!(store.deleted_entries().unwrap().is_empty());
    assert!(store.restore_entry(&entry.id, 2).is_err());
    store.restore_experiment(&experiment.id).unwrap();
    assert!(
        store
            .entries(&experiment.log_project_id, "")
            .unwrap()
            .is_empty()
    );
    assert_eq!(store.deleted_entries().unwrap().len(), 1);
    store.restore_entry(&entry.id, 2).unwrap();
    assert_eq!(store.entry(&entry.id).unwrap().body, entry.body);
}

#[test]
fn schema_three_upgrade_keeps_metadata_notes_and_unknown_experiment_dates() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("crystal-forge.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(include_str!("fixtures/schema-v3.sql"))
        .unwrap();
    connection.execute_batch("INSERT INTO projects VALUES ('log','Old log','Keep','2024-01-01T00:00:00Z'); INSERT INTO cvt_experiments VALUES ('cvt','Old CVT',650,600,'2024-01-02T00:00:00Z','endothermic','Sb + Se','Sb2Se3','I2'); INSERT INTO experiment_logs VALUES ('cvt','log'); INSERT INTO experiment_elements VALUES ('cvt','precursor',51); INSERT INTO entries VALUES ('note','log','Old note','Original content','2020-02-29','2024-01-02T00:00:00Z','2024-01-02T00:00:00Z',4);").unwrap();
    drop(connection);
    let store = Store::open(folder.path()).unwrap();
    let cvt = store.cvt_experiment("cvt").unwrap();
    assert_eq!(cvt.details.experiment_date, None);
    assert_eq!(cvt.details.transport_agent, "I2");
    assert_eq!(cvt.details.precursor.elements, [51].into());
    assert_eq!(store.entry("note").unwrap().revision, 4);
    assert!(store.deleted_entries().unwrap().is_empty());
    assert!(store.deleted_experiments().unwrap().is_empty());
    drop(store);
    let reopened = Store::open(folder.path()).unwrap();
    assert_eq!(reopened.cvt_experiment("cvt").unwrap(), cvt);
}

#[test]
fn whole_log_deletion_is_atomic_scoped_and_rejects_changed_confirmations() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(folder.path()).unwrap();
    let a = store.create_project("First log", "").unwrap();
    let b = store.create_project("Other log", "").unwrap();
    let first = store.create_entry(&note(&a.id)).unwrap();
    let other = store.create_entry(&note(&b.id)).unwrap();
    let expected = vec![(first.id.clone(), first.revision)];
    let second = store.create_entry(&note(&a.id)).unwrap();
    assert!(matches!(
        store.delete_log_entries(&a.id, &expected),
        Err(StoreError::Conflict)
    ));
    assert_eq!(store.entries(&a.id, "").unwrap().len(), 2);
    let all = vec![(first.id.clone(), 1), (second.id.clone(), 1)];
    // A DB write failure after the first row must roll back the entire deletion.
    let connection = rusqlite::Connection::open(store.path()).unwrap();
    connection.execute_batch("CREATE TRIGGER test_reject_deletion BEFORE UPDATE OF deleted_at ON entries WHEN OLD.rowid = (SELECT max(rowid) FROM entries WHERE project_id=OLD.project_id) BEGIN SELECT RAISE(ABORT,'simulated deletion failure'); END;").unwrap();
    assert!(store.delete_log_entries(&a.id, &all).is_err());
    assert_eq!(store.entries(&a.id, "").unwrap().len(), 2);
    assert!(store.deleted_entries().unwrap().is_empty());
    connection
        .execute_batch("DROP TRIGGER test_reject_deletion;")
        .unwrap();
    assert_eq!(store.delete_log_entries(&a.id, &all).unwrap(), 2);
    assert!(store.entries(&a.id, "").unwrap().is_empty());
    assert_eq!(store.entry(&other.id).unwrap(), other);
    let deleted = store.deleted_entries().unwrap();
    assert_eq!(deleted.len(), 2);
    for entry in deleted {
        store.restore_entry(&entry.id, entry.revision).unwrap();
    }
    assert_eq!(store.entries(&a.id, "").unwrap().len(), 2);
}
