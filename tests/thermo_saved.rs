use crystal_forge::store::Store;

#[test]
fn saved_calculation_snapshot_stays_in_its_profile_and_workbook() {
    let folder = tempfile::tempdir().unwrap();
    let first = Store::open(&folder.path().join("profile-a")).unwrap();
    let second = Store::open(&folder.path().join("profile-b")).unwrap();
    let workbook = first.active_workbook_id().unwrap();
    let other = first.create_workbook("Other", "", "", "").unwrap();
    first
        .save_thermo_calculation(
            &workbook,
            "A calculation",
            r#"{"record_revision":2,"delta_g_j_mol":42}"#,
        )
        .unwrap();
    assert_eq!(first.thermo_calculations(&workbook).unwrap().len(), 1);
    assert!(first.thermo_calculations(&other.id).unwrap().is_empty());
    assert!(
        second
            .thermo_calculations(&second.active_workbook_id().unwrap())
            .unwrap()
            .is_empty()
    );
    drop(first);
    let reopened = Store::open(&folder.path().join("profile-a")).unwrap();
    assert_eq!(
        reopened.thermo_calculations(&workbook).unwrap()[0].snapshot_json,
        r#"{"record_revision":2,"delta_g_j_mol":42}"#
    );
}
