//! Profile-local pictures referenced from a note's Markdown source.
use image::ImageDecoder;
use std::{
    io::Cursor,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const MAX_BYTES: u64 = 10 * 1024 * 1024;
const PREFIX: &str = "crystalforge-image:";

pub fn add(workspace: &Path, source: &Path) -> Result<(String, PathBuf), String> {
    if std::fs::metadata(source).map_err(|e| e.to_string())?.len() > MAX_BYTES {
        return Err("Choose a PNG or JPEG smaller than 10 MB.".into());
    }
    let bytes = std::fs::read(source).map_err(|e| e.to_string())?;
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
    ) {
        return Err("Choose a PNG or JPEG picture.".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits.clone());
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    limits
        .reserve(decoder.total_bytes())
        .map_err(|e| e.to_string())?;
    decoder.set_limits(limits).map_err(|e| e.to_string())?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut picture = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    picture.apply_orientation(orientation);
    picture = picture.resize(1600, 1600, image::imageops::FilterType::Lanczos3);
    let mut png = Cursor::new(Vec::new());
    picture
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let folder = workspace.join("note-images");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let path = folder.join(format!("{id}.png"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    use std::io::Write;
    file.write_all(png.get_ref())
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    Ok((format!("![Picture]({PREFIX}{id})"), path))
}

pub fn split_markdown(body: &str) -> (String, Vec<(String, String)>) {
    let mut cleaned = String::new();
    let mut images = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("![") {
        cleaned.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(close_alt) = rest.find("](") else {
            break;
        };
        let Some(close_uri) = rest[close_alt + 2..].find(')') else {
            break;
        };
        let close_uri = close_alt + 2 + close_uri;
        let alt = &rest[2..close_alt];
        let uri = &rest[close_alt + 2..close_uri];
        if let Some(id) = uri
            .strip_prefix(PREFIX)
            .filter(|id| Uuid::parse_str(id).is_ok())
        {
            images.push((alt.to_owned(), id.to_owned()));
        } else {
            cleaned.push_str(&rest[..=close_uri]);
        }
        rest = &rest[close_uri + 1..];
    }
    cleaned.push_str(rest);
    (cleaned, images)
}

pub fn picture_path(workspace: &Path, id: &str) -> Option<PathBuf> {
    Uuid::parse_str(id)
        .ok()
        .map(|_| workspace.join("note-images").join(format!("{id}.png")))
}
