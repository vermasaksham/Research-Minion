//! Offline user identities and separate research workspaces.
use crate::store::{Store, StoreError};
use chrono::{DateTime, Utc};
use image::ImageDecoder;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params, types::Type};
use std::{
    io::Cursor,
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;
use uuid::Uuid;

const MAX_PICTURE_BYTES: usize = 10 * 1024 * 1024;
#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("{0}")]
    Validation(String),
    #[error("This profile is no longer available.")]
    NotFound,
    #[error("This profile catalog is not supported by this version of CrystalForge.")]
    UnsupportedCatalog,
    #[error("Local file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Profile database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("{0}")]
    Workspace(#[from] StoreError),
}
pub type Result<T> = std::result::Result<T, ProfileError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub full_name: String,
    pub picture_png: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    /// The first profile may own the pre-profile database in its original location.
    pub owns_legacy_workspace: bool,
}

pub fn prepare_picture(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.is_empty() || bytes.len() > MAX_PICTURE_BYTES {
        return Err(ProfileError::Validation(
            "Choose a PNG or JPEG picture smaller than 10 MB.".into(),
        ));
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| ProfileError::Validation("The picture could not be read.".into()))?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
    ) {
        return Err(ProfileError::Validation(
            "Choose a PNG or JPEG picture.".into(),
        ));
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits.clone());
    let decode_error = || {
        ProfileError::Validation(
            "This picture is damaged or too large. Choose a smaller PNG or JPEG.".into(),
        )
    };
    let mut decoder = reader.into_decoder().map_err(|_| decode_error())?;
    limits
        .reserve(decoder.total_bytes())
        .map_err(|_| decode_error())?;
    decoder.set_limits(limits).map_err(|_| decode_error())?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut image = image::DynamicImage::from_decoder(decoder).map_err(|_| decode_error())?;
    image.apply_orientation(orientation);
    let avatar = image.resize_to_fill(256, 256, image::imageops::FilterType::Lanczos3);
    let mut output = Cursor::new(Vec::new());
    avatar
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|_| ProfileError::Validation("The profile picture could not be saved.".into()))?;
    Ok(output.into_inner())
}

pub fn picture_from_file(path: &Path) -> Result<Vec<u8>> {
    if std::fs::metadata(path)?.len() > MAX_PICTURE_BYTES as u64 {
        return Err(ProfileError::Validation(
            "Choose a picture smaller than 10 MB.".into(),
        ));
    }
    prepare_picture(&std::fs::read(path)?)
}

pub struct Profiles {
    connection: Connection,
    root: PathBuf,
}
impl Profiles {
    pub fn open(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root)?;
        let mut connection = Connection::open(root.join("profiles.sqlite3"))?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > 1 {
            return Err(ProfileError::UnsupportedCatalog);
        }
        if version == 0 {
            let count: i64 = connection.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )?;
            if count != 0 {
                return Err(ProfileError::UnsupportedCatalog);
            }
            let transaction = connection.transaction()?;
            transaction.execute_batch("CREATE TABLE profiles (
                id TEXT PRIMARY KEY NOT NULL,
                full_name TEXT NOT NULL CHECK(length(trim(full_name))>0),
                picture_png BLOB,
                created_at TEXT NOT NULL,
                owns_legacy INTEGER NOT NULL CHECK(owns_legacy IN (0,1))
            );
            CREATE UNIQUE INDEX one_legacy_owner ON profiles(owns_legacy) WHERE owns_legacy=1;
            CREATE TABLE preferences (id INTEGER PRIMARY KEY CHECK(id=1), active_profile_id TEXT REFERENCES profiles(id));
            INSERT INTO preferences VALUES (1,NULL);
            PRAGMA user_version=1;")?;
            transaction.commit()?;
        }
        connection.prepare(
            "SELECT id,full_name,picture_png,created_at,owns_legacy FROM profiles LIMIT 0",
        )?;
        connection.prepare("SELECT active_profile_id FROM preferences WHERE id=1")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        Ok(Self {
            connection,
            root: root.to_path_buf(),
        })
    }
    pub fn has_unassigned_workspace(&self) -> Result<bool> {
        Ok(self.all()?.is_empty()
            && (self.root.join("crystal-forge.sqlite3").exists()
                || self.root.join("research-minion.sqlite3").exists()))
    }
    pub fn all(&self) -> Result<Vec<Profile>> {
        let mut statement=self.connection.prepare("SELECT id,full_name,picture_png,created_at,owns_legacy FROM profiles ORDER BY created_at,id")?;
        Ok(statement
            .query_map([], read_profile)?
            .collect::<std::result::Result<_, _>>()?)
    }
    pub fn profile(&self, id: &str) -> Result<Profile> {
        self.connection
            .query_row(
                "SELECT id,full_name,picture_png,created_at,owns_legacy FROM profiles WHERE id=?1",
                [id],
                read_profile,
            )
            .optional()?
            .ok_or(ProfileError::NotFound)
    }
    pub fn active(&self) -> Result<Option<Profile>> {
        let id: Option<String> = self.connection.query_row(
            "SELECT active_profile_id FROM preferences WHERE id=1",
            [],
            |row| row.get(0),
        )?;
        id.map(|id| self.profile(&id)).transpose()
    }
    pub fn create(&mut self, full_name: &str, picture: Option<&[u8]>) -> Result<Profile> {
        let name = valid_name(full_name)?;
        let picture_png = picture.map(prepare_picture).transpose()?;
        // Serialize first-profile ownership so two instances cannot claim the old workspace.
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count: i64 =
            transaction.query_row("SELECT count(*) FROM profiles", [], |row| row.get(0))?;
        let profile = Profile {
            id: Uuid::new_v4().to_string(),
            full_name: name,
            picture_png,
            created_at: Utc::now(),
            owns_legacy_workspace: count == 0
                && (self.root.join("crystal-forge.sqlite3").exists()
                    || self.root.join("research-minion.sqlite3").exists()),
        };
        let folder = if profile.owns_legacy_workspace {
            self.root.clone()
        } else {
            self.root.join("profiles").join(&profile.id)
        };
        // Validate/open successfully before publishing a profile in the catalog.
        // No existing research file is copied, renamed, or deleted.
        let _workspace = Store::open(&folder)?;
        transaction.execute(
            "INSERT INTO profiles VALUES (?1,?2,?3,?4,?5)",
            params![
                profile.id,
                profile.full_name,
                profile.picture_png,
                profile.created_at.to_rfc3339(),
                profile.owns_legacy_workspace
            ],
        )?;
        transaction.commit()?;
        Ok(profile)
    }
    pub fn update(&self, id: &str, name: &str, picture: Option<&[u8]>) -> Result<Profile> {
        let name = valid_name(name)?;
        let picture = picture.map(prepare_picture).transpose()?;
        if self.connection.execute(
            "UPDATE profiles SET full_name=?1,picture_png=?2 WHERE id=?3",
            params![name, picture, id],
        )? == 0
        {
            return Err(ProfileError::NotFound);
        }
        self.profile(id)
    }
    pub fn workspace(&self, id: &str) -> Result<Store> {
        let profile = self.profile(id)?;
        let folder = if profile.owns_legacy_workspace {
            self.root.clone()
        } else {
            self.root.join("profiles").join(&profile.id)
        };
        Ok(Store::open(&folder)?)
    }
    pub fn activate(&self, id: &str) -> Result<()> {
        self.profile(id)?;
        self.connection.execute(
            "UPDATE preferences SET active_profile_id=?1 WHERE id=1",
            [id],
        )?;
        Ok(())
    }
}

fn valid_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 200 || name.chars().any(char::is_control) {
        return Err(ProfileError::Validation(
            "Enter your full name (up to 200 characters).".into(),
        ));
    }
    Ok(name.into())
}
fn read_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<Profile> {
    let id: String = row.get(0)?;
    // Stored identifiers can never be used as arbitrary filesystem paths.
    if Uuid::parse_str(&id).is_err() || id.contains(['/', '\\']) {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            Type::Text,
            Box::new(ProfileError::Validation(
                "Invalid saved profile identifier.".into(),
            )),
        ));
    }
    let time: String = row.get(3)?;
    Ok(Profile {
        id,
        full_name: row.get(1)?,
        picture_png: row.get(2)?,
        created_at: time.parse().map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(3, Type::Text, Box::new(error))
        })?,
        owns_legacy_workspace: row.get(4)?,
    })
}
