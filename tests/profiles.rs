use chrono::NaiveDate;
use crystal_forge::{
    cvt::CvtConditions,
    model::EntryDraft,
    profiles::{Profiles, picture_from_file, prepare_picture},
    store::{Store, StoreError},
};
use std::io::Cursor;
fn picture() -> Vec<u8> {
    let image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        24,
        36,
        image::Rgb([24, 112, 109]),
    ));
    let mut cursor = Cursor::new(Vec::new());
    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .unwrap();
    cursor.into_inner()
}
fn note(project_id: &str) -> EntryDraft {
    EntryDraft {
        project_id: project_id.into(),
        title: "Private observation".into(),
        body: "Sb₂Se₃ notes\nOnly this profile".into(),
        event_date: NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
    }
}
#[test]
fn identity_and_copied_picture_survive_restart_without_the_source_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("photo.png");
    std::fs::write(&path, picture()).unwrap();
    let prepared = picture_from_file(&path).unwrap();
    let mut profiles = Profiles::open(dir.path()).unwrap();
    assert!(profiles.active().unwrap().is_none());
    let profile = profiles
        .create("  René Example  ", Some(&prepared))
        .unwrap();
    profiles.activate(&profile.id).unwrap();
    assert_eq!(profile.full_name, "René Example");
    assert!(!profile.owns_legacy_workspace);
    assert!(!dir.path().join("crystal-forge.sqlite3").exists());
    assert_eq!(
        profiles
            .workspace(&profile.id)
            .unwrap()
            .path()
            .file_name()
            .unwrap(),
        "crystal-forge.sqlite3"
    );
    assert_eq!(
        image::load_from_memory(profile.picture_png.as_ref().unwrap())
            .unwrap()
            .width(),
        256
    );
    std::fs::remove_file(path).unwrap();
    drop(profiles);
    let reopened = Profiles::open(dir.path()).unwrap();
    assert_eq!(reopened.active().unwrap(), Some(profile));
}
#[test]
fn profiles_keep_experiments_notes_search_and_deleted_records_in_separate_databases() {
    let dir = tempfile::tempdir().unwrap();
    let mut profiles = Profiles::open(dir.path()).unwrap();
    let a = profiles.create("Researcher A", None).unwrap();
    let b = profiles.create("Researcher B", None).unwrap();
    let first = profiles.workspace(&a.id).unwrap();
    let second = profiles.workspace(&b.id).unwrap();
    assert_ne!(first.path(), second.path());
    let cvt = first
        .create_cvt_experiment("Shared label", CvtConditions::new(650., 600.).unwrap())
        .unwrap();
    let entry = first.create_entry(&note(&cvt.log_project_id)).unwrap();
    assert!(second.cvt_experiments().unwrap().is_empty());
    assert!(second.projects().unwrap().is_empty());
    assert!(
        second
            .entries(&cvt.log_project_id, "Sb₂Se₃")
            .unwrap()
            .is_empty()
    );
    assert!(matches!(second.entry(&entry.id), Err(StoreError::NotFound)));
    assert!(second.create_entry(&note(&cvt.log_project_id)).is_err());
    assert!(second.delete_experiment(&cvt.id).is_err());
    assert!(second.delete_entry(&entry.id, 1).is_err());
    first.delete_entry(&entry.id, 1).unwrap();
    assert!(second.deleted_entries().unwrap().is_empty());
    first.delete_experiment(&cvt.id).unwrap();
    assert!(second.deleted_experiments().unwrap().is_empty());
    profiles.activate(&b.id).unwrap();
    assert_eq!(profiles.active().unwrap().unwrap().id, b.id);
    profiles.activate(&a.id).unwrap();
    assert_eq!(
        profiles
            .workspace(&a.id)
            .unwrap()
            .deleted_experiments()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn first_profile_claims_existing_workspace_in_place_and_later_profiles_start_empty() {
    let dir = tempfile::tempdir().unwrap();
    let old = Store::open(dir.path()).unwrap();
    let cvt = old
        .create_cvt_experiment("Earlier research", CvtConditions::new(500., 550.).unwrap())
        .unwrap();
    let entry = old.create_entry(&note(&cvt.log_project_id)).unwrap();
    let legacy_path = old.path().to_path_buf();
    let mut profiles = Profiles::open(dir.path()).unwrap();
    assert!(profiles.has_unassigned_workspace().unwrap());
    let mut concurrent = Profiles::open(dir.path()).unwrap();
    let first = profiles.create("Original owner", None).unwrap();
    let second = concurrent.create("New owner", None).unwrap();
    assert!(first.owns_legacy_workspace);
    assert!(!second.owns_legacy_workspace);
    assert_eq!(profiles.workspace(&first.id).unwrap().path(), legacy_path);
    assert_eq!(
        profiles
            .workspace(&first.id)
            .unwrap()
            .entry(&entry.id)
            .unwrap(),
        entry
    );
    assert_eq!(
        profiles
            .workspace(&first.id)
            .unwrap()
            .cvt_experiment(&cvt.id)
            .unwrap(),
        cvt
    );
    assert!(
        profiles
            .workspace(&second.id)
            .unwrap()
            .cvt_experiments()
            .unwrap()
            .is_empty()
    );
    assert_eq!(old.entry(&entry.id).unwrap(), entry); // No move/copy while another instance still holds WAL.
    assert!(!profiles.has_unassigned_workspace().unwrap());
}
#[test]
fn invalid_identity_picture_and_missing_activation_leave_existing_data_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let mut profiles = Profiles::open(dir.path()).unwrap();
    for name in ["", "  ", "Name\nInjected"] {
        assert!(profiles.create(name, None).is_err());
    }
    assert!(profiles.create(&"a".repeat(201), None).is_err());
    assert!(
        profiles
            .create("Valid name", Some(b"not a picture"))
            .is_err()
    );
    assert!(profiles.all().unwrap().is_empty());
    let profile = profiles.create("Valid owner", None).unwrap();
    profiles.activate(&profile.id).unwrap();
    assert!(profiles.activate("missing").is_err());
    assert_eq!(profiles.active().unwrap().unwrap(), profile);
    assert!(prepare_picture(&vec![0; 10 * 1024 * 1024 + 1]).is_err());
    assert!(prepare_picture(b"GIF89a").is_err());
}
#[test]
fn changing_name_or_picture_keeps_workspace_identity_and_research_records() {
    let dir = tempfile::tempdir().unwrap();
    let mut profiles = Profiles::open(dir.path()).unwrap();
    let original = profiles.create("Before", None).unwrap();
    let store = profiles.workspace(&original.id).unwrap();
    let experiment = store
        .create_cvt_experiment("Stable record", CvtConditions::new(650., 600.).unwrap())
        .unwrap();
    let changed = profiles
        .update(&original.id, "After", Some(&picture()))
        .unwrap();
    assert_eq!(changed.id, original.id);
    assert_eq!(changed.created_at, original.created_at);
    assert!(changed.picture_png.is_some());
    assert_eq!(
        profiles
            .workspace(&changed.id)
            .unwrap()
            .cvt_experiment(&experiment.id)
            .unwrap(),
        experiment
    );
    assert!(
        profiles
            .update(&original.id, "After", None)
            .unwrap()
            .picture_png
            .is_none()
    );
}
#[test]
fn unsupported_catalog_and_unrecognised_legacy_database_are_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profiles.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TABLE future_data (value TEXT); INSERT INTO future_data VALUES ('keep'); PRAGMA user_version=42;").unwrap();
    drop(connection);
    let before = std::fs::read(&path).unwrap();
    assert!(Profiles::open(dir.path()).is_err());
    assert_eq!(std::fs::read(path).unwrap(), before);
    let other = tempfile::tempdir().unwrap();
    let path = other.path().join("research-minion.sqlite3");
    std::fs::write(&path, b"Not a research database").unwrap();
    let mut profiles = Profiles::open(other.path()).unwrap();
    assert!(profiles.create("Owner", None).is_err());
    assert!(profiles.all().unwrap().is_empty());
    assert_eq!(std::fs::read(path).unwrap(), b"Not a research database");
}
#[test]
fn corrupt_profile_identifiers_cannot_become_workspace_paths() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = Profiles::open(dir.path()).unwrap();
    let connection = rusqlite::Connection::open(dir.path().join("profiles.sqlite3")).unwrap();
    connection
        .execute(
            "INSERT INTO profiles VALUES ('../escape','Invalid',NULL,'2024-01-01T00:00:00Z',0)",
            [],
        )
        .unwrap();
    assert!(profiles.workspace("../escape").is_err());
    assert!(profiles.all().is_err());
    assert!(!dir.path().join("escape").exists());
}

#[test]
fn jpeg_pictures_normalise_to_png_and_oversized_dimensions_are_rejected() {
    let image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        40,
        20,
        image::Rgb([48, 128, 176]),
    ));
    let mut jpeg = Cursor::new(Vec::new());
    image.write_to(&mut jpeg, image::ImageFormat::Jpeg).unwrap();
    let png = prepare_picture(jpeg.get_ref()).unwrap();
    assert_eq!(image::guess_format(&png).unwrap(), image::ImageFormat::Png);
    let restored = image::load_from_memory(&png).unwrap();
    assert_eq!((restored.width(), restored.height()), (256, 256));
    let too_wide = image::DynamicImage::ImageRgb8(image::RgbImage::new(9000, 1));
    let mut bytes = Cursor::new(Vec::new());
    too_wide
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    assert!(prepare_picture(bytes.get_ref()).is_err());
}

#[test]
fn phone_photo_orientation_is_applied_before_the_square_avatar_crop() {
    // Synthetic red/blue image with EXIF orientation 6 (90° clockwise).
    let png = prepare_picture(include_bytes!("fixtures/profile-orientation.jpg")).unwrap();
    let avatar = image::load_from_memory(&png).unwrap().to_rgb8();
    let top = avatar.get_pixel(128, 32);
    let bottom = avatar.get_pixel(128, 224);
    assert!(top[0] > 200 && top[2] < 80, "top should be red: {top:?}");
    assert!(
        bottom[2] > 200 && bottom[0] < 80,
        "bottom should be blue: {bottom:?}"
    );
}
