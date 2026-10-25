use chrono::NaiveDate;
use research_minion::{
    editor::Editor,
    model::EntryDraft,
    store::{Store, StoreError},
};

fn draft(project_id: &str, title: &str, body: &str, date: &str) -> EntryDraft {
    EntryDraft {
        project_id: project_id.into(),
        title: title.into(),
        body: body.into(),
        event_date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
    }
}

#[test]
fn project_and_scientific_notes_survive_close_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let (project_id, entry_id, created_at) = {
        let store = Store::open(dir.path()).unwrap();
        let project = store
            .create_project("  Sb₂Se₃ growth  ", "CVT studies")
            .unwrap();
        assert_eq!(project.name, "Sb₂Se₃ growth");
        let entry = store
            .create_entry(&draft(
                &project.id,
                "CVT trial",
                "Source 550 °C; sink 500 °C\nPressure: 10⁻³ mbar\nInterpretation: unassigned",
                "2020-02-29",
            ))
            .unwrap();
        assert_ne!(entry.id, project.id);
        assert_eq!(entry.revision, 1);
        assert_eq!(entry.created_at, entry.updated_at);
        assert!(entry.created_at.date_naive() > entry.event_date);
        (project.id, entry.id, entry.created_at)
    };
    let store = Store::open(dir.path()).unwrap();
    let projects = store.projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, project_id);
    assert_eq!(projects[0].description, "CVT studies");
    let entry = store.entry(&entry_id).unwrap();
    assert_eq!(entry.created_at, created_at);
    assert_eq!(
        entry.body,
        "Source 550 °C; sink 500 °C\nPressure: 10⁻³ mbar\nInterpretation: unassigned"
    );
    assert_eq!(store.entries(&project_id, "").unwrap(), vec![entry]);
}

#[test]
fn editing_keeps_identity_creation_time_and_project() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let project = store.create_project("Growth", "").unwrap();
    let original = store
        .create_entry(&draft(
            &project.id,
            "Trial",
            "Initial observation",
            "2020-01-01",
        ))
        .unwrap();
    let updated = store
        .update_entry(
            &original.id,
            original.revision,
            &draft(
                &project.id,
                "  Revised trial  ",
                "Revised interpretation",
                "2020-01-02",
            ),
        )
        .unwrap();
    assert_eq!(updated.id, original.id);
    assert_eq!(updated.created_at, original.created_at);
    assert_eq!(updated.project_id, original.project_id);
    assert_eq!(updated.revision, 2);
    assert_eq!(updated.title, "Revised trial");
    assert_eq!(updated.event_date.to_string(), "2020-01-02");
    assert!(updated.updated_at >= original.updated_at);
}

#[test]
fn search_is_literal_case_insensitive_and_scoped_to_one_project() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let first = store.create_project("First", "").unwrap();
    let other = store.create_project("Other", "").unwrap();
    let match_entry = store
        .create_entry(&draft(
            &first.id,
            "Sb₂Se₃ CVT",
            "Änderung: 20%_yield; 'quoted'",
            "2020-01-01",
        ))
        .unwrap();
    store
        .create_entry(&draft(&first.id, "SEM", "Unrelated", "2020-01-02"))
        .unwrap();
    store
        .create_entry(&draft(&other.id, "Sb₂Se₃ CVT", "Änderung", "2020-01-01"))
        .unwrap();
    for query in ["cvt", "SB₂SE₃", "änderung", "%_", "'quoted'"] {
        assert_eq!(
            store.entries(&first.id, query).unwrap(),
            vec![match_entry.clone()],
            "query: {query}"
        );
    }
    assert!(store.entries(&first.id, "' OR 1=1 --").unwrap().is_empty());
    assert_eq!(store.entries(&first.id, "  ").unwrap().len(), 2);
}

#[test]
fn entries_are_sorted_by_event_date_rather_than_documentation_date() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let project = store.create_project("Growth", "").unwrap();
    let recent = store
        .create_entry(&draft(&project.id, "Recent event", "", "2024-06-01"))
        .unwrap();
    let retrospective = store
        .create_entry(&draft(
            &project.id,
            "Past event, entered later",
            "",
            "2020-01-01",
        ))
        .unwrap();
    assert_eq!(
        store.entries(&project.id, "").unwrap(),
        vec![recent, retrospective]
    );
}

#[test]
fn invalid_records_are_rejected_without_partial_writes() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    assert!(matches!(
        store.create_project(" \t\n ", ""),
        Err(StoreError::Validation(_))
    ));
    let project = store.create_project("Growth", "").unwrap();
    assert!(matches!(
        store.create_entry(&draft(&project.id, "\n  ", "Notes", "2020-01-01")),
        Err(StoreError::Validation(_))
    ));
    assert!(matches!(
        store.create_entry(&draft("missing-project", "Title", "", "2020-01-01")),
        Err(StoreError::Validation(_))
    ));
    assert!(store.entries(&project.id, "").unwrap().is_empty());
    assert_eq!(store.projects().unwrap().len(), 1);
}

#[test]
fn second_instance_cannot_silently_overwrite_a_newer_revision() {
    let dir = tempfile::tempdir().unwrap();
    let first = Store::open(dir.path()).unwrap();
    let project = first.create_project("Growth", "").unwrap();
    let original = first
        .create_entry(&draft(&project.id, "Trial", "Before", "2020-01-01"))
        .unwrap();
    let second = Store::open(dir.path()).unwrap();
    let mut stale_editor = Editor::from_entry(second.entry(&original.id).unwrap());
    stale_editor.body = "Unsaved second interpretation".into();
    let updated = first
        .update_entry(
            &original.id,
            1,
            &draft(
                &project.id,
                "Trial",
                "Saved first interpretation",
                "2020-01-01",
            ),
        )
        .unwrap();
    assert!(matches!(
        stale_editor.save(&second),
        Err(StoreError::Conflict)
    ));
    assert_eq!(stale_editor.body, "Unsaved second interpretation");
    assert!(stale_editor.is_dirty());
    assert_eq!(second.entry(&original.id).unwrap(), updated);
}

#[test]
fn editor_marks_saved_baseline_and_keeps_failed_edits() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let project = store.create_project("Growth", "").unwrap();
    let mut editor = Editor::new(project.id.clone());
    assert!(!editor.is_dirty());
    editor.body = "Important notes without a title".into();
    assert!(editor.is_dirty());
    assert!(matches!(
        editor.save(&store),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(editor.body, "Important notes without a title");
    editor.title = "  First entry  ".into();
    editor.event_date = "2020-02-29".into();
    editor.save(&store).unwrap();
    assert!(!editor.is_dirty());
    assert_eq!(editor.title, "First entry");
    let id = editor.saved.as_ref().unwrap().id.clone();
    editor.body = "Edited notes".into();
    editor.save(&store).unwrap();
    assert_eq!(store.entry(&id).unwrap().body, "Edited notes");
    assert_eq!(store.entries(&project.id, "").unwrap().len(), 1);
    assert!(!editor.is_dirty());
}

#[test]
fn malformed_dates_never_replace_saved_data() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let project = store.create_project("Growth", "").unwrap();
    let original = store
        .create_entry(&draft(&project.id, "Trial", "Original", "2020-01-01"))
        .unwrap();
    let mut editor = Editor::from_entry(original.clone());
    for date in ["2023-02-29", "01/02/2024", "2024-1-2", "", "2024-13-01"] {
        editor.event_date = date.into();
        assert!(
            matches!(editor.save(&store), Err(StoreError::Validation(_))),
            "date: {date}"
        );
        assert!(editor.is_dirty());
        assert_eq!(store.entry(&original.id).unwrap(), original);
    }
}

#[test]
fn newer_database_schema_is_rejected_without_modification() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("research-minion.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TABLE future_data (value TEXT); INSERT INTO future_data VALUES ('keep me'); PRAGMA user_version = 42;").unwrap();
    drop(connection);
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        Store::open(dir.path()),
        Err(StoreError::UnsupportedVersion(42))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn unrelated_database_is_not_adopted_or_modified() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("research-minion.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch("CREATE TABLE existing_work (value TEXT);")
        .unwrap();
    drop(connection);
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        Store::open(dir.path()),
        Err(StoreError::UnrecognisedDatabase)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn nested_data_folder_and_missing_entry_are_handled() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("local workspace").join("data");
    let store = Store::open(&nested).unwrap();
    assert!(store.path().exists());
    assert!(matches!(
        store.entry("not-an-entry"),
        Err(StoreError::NotFound)
    ));
}
