use crystal_forge::note_images;
use std::io::Cursor;

#[test]
fn pictures_are_profile_local_and_reopen_from_saved_note_markdown() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let input = root.path().join("microscope.png");
    let picture = image::DynamicImage::ImageRgb8(image::RgbImage::new(32, 20));
    let mut png = Cursor::new(Vec::new());
    picture.write_to(&mut png, image::ImageFormat::Png).unwrap();
    std::fs::write(&input, png.into_inner()).unwrap();
    let (reference, path) = note_images::add(root.path(), &input).unwrap();
    assert!(path.exists());
    let (text, pictures) = note_images::split_markdown(&format!("Before\n{reference}\nAfter"));
    assert_eq!(text, "Before\n\nAfter");
    assert_eq!(pictures.len(), 1);
    assert_eq!(
        note_images::picture_path(root.path(), &pictures[0].1),
        Some(path)
    );
    assert!(
        !note_images::picture_path(other.path(), &pictures[0].1)
            .unwrap()
            .exists()
    );
    assert_eq!(
        note_images::split_markdown("![x](file:///tmp/secret.png)").0,
        "![x](file:///tmp/secret.png)"
    );
    assert!(note_images::picture_path(root.path(), "../escape").is_none());
}
