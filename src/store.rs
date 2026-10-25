use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row, params, types::Type};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    cvt::CvtConditions,
    model::{CvtExperiment, Entry, EntryDraft, Project},
};

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("{0}")]
    Validation(String),
    #[error(
        "This entry changed in another instance. Your text is still in the editor. Copy it before reloading the saved entry."
    )]
    Conflict,
    #[error("The requested record no longer exists.")]
    NotFound,
    #[error(
        "Database version {0} is newer than this application supports. Use a newer Research Minion."
    )]
    UnsupportedVersion(i64),
    #[error("This is not an empty or recognised Research Minion database.")]
    UnrecognisedDatabase,
    #[error("The local data directory is unavailable. Choose a location with --data-dir.")]
    NoDataDirectory,
    #[error("Local file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Local database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// Each store owns one connection; no server or network access is required.
pub struct Store {
    connection: Connection,
    path: PathBuf,
}

impl Store {
    pub fn open(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        Self::open_file(&data_dir.join("research-minion.sqlite3"))
    }

    pub fn open_file(path: &Path) -> Result<Self> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        // Check before making schema or journal changes. Never downgrade a file.
        let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > 2 {
            return Err(StoreError::UnsupportedVersion(version));
        }
        if version == 0 {
            let table_count: i64 = connection.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
                [],
                |r| r.get(0),
            )?;
            if table_count != 0 {
                return Err(StoreError::UnrecognisedDatabase);
            }
            let transaction = connection.transaction()?;
            transaction.execute_batch(
                "CREATE TABLE projects (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
                    description TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE entries (
                    id TEXT PRIMARY KEY NOT NULL,
                    project_id TEXT NOT NULL REFERENCES projects(id),
                    title TEXT NOT NULL CHECK(length(trim(title)) > 0),
                    body TEXT NOT NULL,
                    event_date TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    revision INTEGER NOT NULL CHECK(revision >= 1)
                );
                CREATE INDEX entries_project_date ON entries(project_id, event_date DESC, updated_at DESC);
                PRAGMA user_version = 1;",
            )?;
            transaction.commit()?;
        }
        // Verify the existing logbook before adding tables to a version-1 file.
        // A version number alone does not establish that this is our schema.
        connection.prepare("SELECT id, name, description, created_at FROM projects LIMIT 0")?;
        connection.prepare("SELECT id, project_id, title, body, event_date, created_at, updated_at, revision FROM entries LIMIT 0")?;
        if version < 2 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(
                "CREATE TABLE cvt_experiments (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
                    source_c REAL NOT NULL CHECK(source_c >= -273.15 AND source_c <= 1.7976931348623157e308),
                    sink_c REAL NOT NULL CHECK(sink_c >= -273.15 AND sink_c <= 1.7976931348623157e308),
                    created_at TEXT NOT NULL
                );
                CREATE INDEX cvt_experiments_created ON cvt_experiments(created_at DESC, id);
                PRAGMA user_version = 2;",
            )?;
            transaction.commit()?;
        }
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        // Probe the new schema too so incompatible files fail at startup.
        connection.prepare(
            "SELECT id, name, source_c, sink_c, created_at FROM cvt_experiments LIMIT 0",
        )?;
        Ok(Self {
            connection,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn create_cvt_experiment(
        &self,
        name: &str,
        conditions: CvtConditions,
    ) -> Result<CvtExperiment> {
        let id = Uuid::new_v4().to_string();
        let experiment = CvtExperiment {
            name: if name.trim().is_empty() {
                format!("CVT {}", &id[..8])
            } else {
                name.trim().into()
            },
            id,
            conditions,
            created_at: Utc::now(),
        };
        self.connection.execute(
            "INSERT INTO cvt_experiments (id, name, source_c, sink_c, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![experiment.id, experiment.name, conditions.source_c(), conditions.sink_c(), experiment.created_at.to_rfc3339()],
        )?;
        Ok(experiment)
    }

    pub fn cvt_experiments(&self) -> Result<Vec<CvtExperiment>> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, source_c, sink_c, created_at FROM cvt_experiments ORDER BY created_at DESC, id",
        )?;
        let rows = statement.query_map([], |row| {
            let conditions = CvtConditions::new(row.get(2)?, row.get(3)?).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(2, Type::Real, Box::new(error))
            })?;
            Ok(CvtExperiment {
                id: row.get(0)?,
                name: row.get(1)?,
                conditions,
                created_at: parse_value(row, 4)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn create_project(&self, name: &str, description: &str) -> Result<Project> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StoreError::Validation("Give the project a name.".into()));
        }
        let project = Project {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            created_at: Utc::now(),
        };
        self.connection.execute(
            "INSERT INTO projects (id, name, description, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                project.id,
                project.name,
                project.description,
                project.created_at.to_rfc3339()
            ],
        )?;
        Ok(project)
    }

    pub fn projects(&self) -> Result<Vec<Project>> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, description, created_at FROM projects ORDER BY created_at, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: parse_value(row, 3)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn entry(&self, id: &str) -> Result<Entry> {
        self.connection.query_row(
            "SELECT id, project_id, title, body, event_date, created_at, updated_at, revision FROM entries WHERE id = ?1",
            [id], read_entry,
        ).optional()?.ok_or(StoreError::NotFound)
    }

    /// Literal, Unicode-aware, case-insensitive substring search in the current
    /// project. Linear scanning is intentional for this small first milestone.
    pub fn entries(&self, project_id: &str, search: &str) -> Result<Vec<Entry>> {
        let mut statement = self.connection.prepare(
            "SELECT id, project_id, title, body, event_date, created_at, updated_at, revision
             FROM entries WHERE project_id = ?1 ORDER BY event_date DESC, updated_at DESC, id",
        )?;
        let rows = statement.query_map([project_id], read_entry)?;
        let needle = search.trim().to_lowercase();
        let mut entries = Vec::new();
        for entry in rows {
            let entry = entry?;
            if needle.is_empty()
                || entry.title.to_lowercase().contains(&needle)
                || entry.body.to_lowercase().contains(&needle)
            {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    pub fn create_entry(&self, draft: &EntryDraft) -> Result<Entry> {
        self.validate_draft(draft)?;
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        self.connection.execute(
            "INSERT INTO entries (id, project_id, title, body, event_date, created_at, updated_at, revision)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 1)",
            params![id, draft.project_id, draft.title.trim(), draft.body, draft.event_date.to_string(), now],
        )?;
        self.entry(&id)
    }

    pub fn update_entry(
        &self,
        id: &str,
        expected_revision: i64,
        draft: &EntryDraft,
    ) -> Result<Entry> {
        self.validate_draft(draft)?;
        let changed = self.connection.execute(
            "UPDATE entries SET title = ?1, body = ?2, event_date = ?3, updated_at = ?4, revision = revision + 1
             WHERE id = ?5 AND revision = ?6 AND project_id = ?7",
            params![draft.title.trim(), draft.body, draft.event_date.to_string(),
                Utc::now().to_rfc3339(), id, expected_revision, draft.project_id],
        )?;
        if changed == 0 {
            return match self.entry(id) {
                Ok(_) => Err(StoreError::Conflict),
                Err(error) => Err(error),
            };
        }
        self.entry(id)
    }

    fn validate_draft(&self, draft: &EntryDraft) -> Result<()> {
        if draft.title.trim().is_empty() {
            return Err(StoreError::Validation(
                "Give the entry a title before saving.".into(),
            ));
        }
        let exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1)",
            [&draft.project_id],
            |r| r.get(0),
        )?;
        if !exists {
            return Err(StoreError::Validation("Choose an existing project.".into()));
        }
        Ok(())
    }
}

pub fn default_data_dir() -> Result<PathBuf> {
    dirs::data_local_dir()
        .map(|p| p.join("ResearchMinion"))
        .ok_or(StoreError::NoDataDirectory)
}

fn read_entry(row: &Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: row.get(0)?,
        project_id: row.get(1)?,
        title: row.get(2)?,
        body: row.get(3)?,
        event_date: parse_value(row, 4)?,
        created_at: parse_value(row, 5)?,
        updated_at: parse_value(row, 6)?,
        revision: row.get(7)?,
    })
}

fn parse_value<T>(row: &Row<'_>, index: usize) -> rusqlite::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let value: String = row.get(index)?;
    value
        .parse()
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(e)))
}
