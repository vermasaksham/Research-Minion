use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{NaiveDate, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params, types::Type};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    chemistry::element,
    cvt::CvtConditions,
    model::{CvtExperiment, Entry, EntryDraft, ExperimentDetails, HeatEffect, Material, Project},
};

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("{0}")]
    Validation(String),
    #[error(
        "The saved record or log changed in another instance. Your unsaved text is still available. Reload and review the latest records before trying again."
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
        if version > 4 {
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
        connection.prepare(
            "SELECT id, name, source_c, sink_c, created_at FROM cvt_experiments LIMIT 0",
        )?;
        if version < 3 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(
                "ALTER TABLE cvt_experiments ADD COLUMN heat_effect TEXT NOT NULL DEFAULT 'unspecified' CHECK(heat_effect IN ('unspecified','endothermic','exothermic'));
                ALTER TABLE cvt_experiments ADD COLUMN precursor_text TEXT NOT NULL DEFAULT '';
                ALTER TABLE cvt_experiments ADD COLUMN product_text TEXT NOT NULL DEFAULT '';
                ALTER TABLE cvt_experiments ADD COLUMN transport_agent TEXT NOT NULL DEFAULT '';
                CREATE TABLE experiment_logs (
                    experiment_id TEXT PRIMARY KEY NOT NULL REFERENCES cvt_experiments(id),
                    project_id TEXT UNIQUE NOT NULL REFERENCES projects(id)
                );
                CREATE TABLE experiment_elements (
                    experiment_id TEXT NOT NULL REFERENCES cvt_experiments(id),
                    role TEXT NOT NULL CHECK(role IN ('precursor','product')),
                    atomic_number INTEGER NOT NULL CHECK(atomic_number BETWEEN 1 AND 118),
                    PRIMARY KEY(experiment_id, role, atomic_number)
                );",
            )?;
            let existing = {
                let mut statement = transaction.prepare("SELECT id, name FROM cvt_experiments")?;
                statement
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?
            };
            for (id, name) in existing {
                let project_id = Uuid::new_v4().to_string();
                transaction.execute("INSERT INTO projects (id,name,description,created_at) VALUES (?1,?2,'CVT experiment log',?3)", params![project_id,name,Utc::now().to_rfc3339()])?;
                transaction.execute(
                    "INSERT INTO experiment_logs (experiment_id,project_id) VALUES (?1,?2)",
                    params![id, project_id],
                )?;
            }
            transaction.pragma_update(None, "user_version", 3)?;
            transaction.commit()?;
        }
        connection.prepare("SELECT heat_effect, precursor_text, product_text, transport_agent FROM cvt_experiments LIMIT 0")?;
        connection.prepare("SELECT experiment_id, project_id FROM experiment_logs LIMIT 0")?;
        connection.prepare(
            "SELECT experiment_id, role, atomic_number FROM experiment_elements LIMIT 0",
        )?;
        if version < 4 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(
                "ALTER TABLE cvt_experiments ADD COLUMN experiment_date TEXT;
                ALTER TABLE cvt_experiments ADD COLUMN deleted_at TEXT;
                ALTER TABLE entries ADD COLUMN deleted_at TEXT;
                CREATE INDEX entries_deleted ON entries(project_id, deleted_at);
                PRAGMA user_version = 4;",
            )?;
            transaction.commit()?;
        }
        connection.prepare("SELECT experiment_date, deleted_at FROM cvt_experiments LIMIT 0")?;
        connection.prepare("SELECT deleted_at FROM entries LIMIT 0")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
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
        self.create_cvt_experiment_with_details(name, conditions, &ExperimentDetails::default())
    }

    pub fn create_cvt_experiment_with_details(
        &self,
        name: &str,
        conditions: CvtConditions,
        details: &ExperimentDetails,
    ) -> Result<CvtExperiment> {
        if details
            .precursor
            .elements
            .iter()
            .chain(&details.product.elements)
            .any(|number| element(*number).is_none())
        {
            return Err(StoreError::Validation(
                "Choose valid elements from the periodic table.".into(),
            ));
        }
        let id = Uuid::new_v4().to_string();
        let experiment = CvtExperiment {
            name: if name.trim().is_empty() {
                format!("CVT {}", &id[..8])
            } else {
                name.trim().into()
            },
            id,
            conditions,
            details: details.clone(),
            log_project_id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
        };
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute("INSERT INTO projects (id,name,description,created_at) VALUES (?1,?2,'CVT experiment log',?3)", params![experiment.log_project_id,experiment.name,experiment.created_at.to_rfc3339()])?;
        transaction.execute(
            "INSERT INTO cvt_experiments (id, name, source_c, sink_c, created_at, heat_effect, precursor_text, product_text, transport_agent, experiment_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![experiment.id, experiment.name, conditions.source_c(), conditions.sink_c(), experiment.created_at.to_rfc3339(),details.heat_effect.as_str(),details.precursor.text,details.product.text,details.transport_agent,details.experiment_date.map(|date| date.to_string())],
        )?;
        transaction.execute(
            "INSERT INTO experiment_logs (experiment_id,project_id) VALUES (?1,?2)",
            params![experiment.id, experiment.log_project_id],
        )?;
        for (role, material) in [
            ("precursor", &details.precursor),
            ("product", &details.product),
        ] {
            for number in &material.elements {
                transaction.execute("INSERT INTO experiment_elements (experiment_id,role,atomic_number) VALUES (?1,?2,?3)",params![experiment.id,role,number])?;
            }
        }
        transaction.commit()?;
        Ok(experiment)
    }

    pub fn cvt_experiments(&self) -> Result<Vec<CvtExperiment>> {
        let mut statement = self.connection.prepare(
            "SELECT c.id,c.name,c.source_c,c.sink_c,c.created_at,c.heat_effect,c.precursor_text,c.product_text,c.transport_agent,l.project_id,c.experiment_date
             FROM cvt_experiments c LEFT JOIN experiment_logs l ON c.id=l.experiment_id WHERE c.deleted_at IS NULL ORDER BY c.created_at DESC,c.id",
        )?;
        let rows = statement.query_map([], read_cvt_experiment)?;
        let mut experiments = Vec::new();
        for row in rows {
            let mut experiment = row?;
            self.load_elements(&mut experiment)?;
            experiments.push(experiment);
        }
        Ok(experiments)
    }

    pub fn cvt_experiment(&self, id: &str) -> Result<CvtExperiment> {
        let mut experiment = self.connection.query_row(
            "SELECT c.id,c.name,c.source_c,c.sink_c,c.created_at,c.heat_effect,c.precursor_text,c.product_text,c.transport_agent,l.project_id,c.experiment_date
             FROM cvt_experiments c LEFT JOIN experiment_logs l ON c.id=l.experiment_id WHERE c.id=?1 AND c.deleted_at IS NULL",[id],read_cvt_experiment,
        ).optional()?.ok_or(StoreError::NotFound)?;
        self.load_elements(&mut experiment)?;
        Ok(experiment)
    }

    /// Hide the experiment as one unit; linked notes and composition remain intact.
    pub fn delete_experiment(&self, id: &str) -> Result<()> {
        let changed = self.connection.execute(
            "UPDATE cvt_experiments SET deleted_at=?1 WHERE id=?2 AND deleted_at IS NULL",
            params![Utc::now().to_rfc3339(), id],
        )?;
        if changed == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub fn restore_experiment(&self, id: &str) -> Result<()> {
        let changed = self.connection.execute(
            "UPDATE cvt_experiments SET deleted_at=NULL WHERE id=?1 AND deleted_at IS NOT NULL",
            [id],
        )?;
        if changed == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub fn deleted_experiments(&self) -> Result<Vec<CvtExperiment>> {
        let mut statement = self.connection.prepare(
            "SELECT c.id,c.name,c.source_c,c.sink_c,c.created_at,c.heat_effect,c.precursor_text,c.product_text,c.transport_agent,l.project_id,c.experiment_date
             FROM cvt_experiments c LEFT JOIN experiment_logs l ON c.id=l.experiment_id WHERE c.deleted_at IS NOT NULL ORDER BY c.deleted_at DESC,c.id",
        )?;
        let rows = statement.query_map([], read_cvt_experiment)?;
        let mut records = Vec::new();
        for row in rows {
            let mut record = row?;
            self.load_elements(&mut record)?;
            records.push(record);
        }
        Ok(records)
    }

    /// Revision-checked, recoverable deletion. A stale editor cannot remove a newer note.
    pub fn delete_entry(&self, id: &str, expected_revision: i64) -> Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        let entry = self.entry(id)?;
        if entry.revision != expected_revision {
            return Err(StoreError::Conflict);
        }
        let changed = transaction.execute(
            "UPDATE entries SET deleted_at=?1, revision=revision+1 WHERE id=?2 AND revision=?3 AND deleted_at IS NULL",
            params![Utc::now().to_rfc3339(), id, expected_revision],
        )?;
        if changed == 0 {
            return Err(StoreError::Conflict);
        }
        transaction.commit()?;
        Ok(())
    }

    /// Delete a confirmed snapshot of a whole log atomically. New or changed
    /// entries invalidate the confirmation instead of silently widening its scope.
    pub fn delete_log_entries(
        &self,
        project_id: &str,
        expected: &[(String, i64)],
    ) -> Result<usize> {
        let transaction = self.connection.unchecked_transaction()?;
        let mut actual: Vec<_> = self
            .entries(project_id, "")?
            .into_iter()
            .map(|entry| (entry.id, entry.revision))
            .collect();
        let mut expected = expected.to_vec();
        actual.sort();
        expected.sort();
        if actual != expected {
            return Err(StoreError::Conflict);
        }
        let changed = transaction.execute(
            "UPDATE entries SET deleted_at=?1, revision=revision+1 WHERE project_id=?2 AND deleted_at IS NULL",
            params![Utc::now().to_rfc3339(),project_id],
        )?;
        if changed != expected.len() {
            return Err(StoreError::Conflict);
        }
        transaction.commit()?;
        Ok(changed)
    }

    /// Only explicitly deleted entries in active logs; hidden experiments restore first.
    pub fn deleted_entries(&self) -> Result<Vec<Entry>> {
        let mut statement = self.connection.prepare(
            "SELECT id,project_id,title,body,event_date,created_at,updated_at,revision FROM entries
             WHERE deleted_at IS NOT NULL AND NOT EXISTS(SELECT 1 FROM experiment_logs l JOIN cvt_experiments c ON l.experiment_id=c.id WHERE l.project_id=entries.project_id AND c.deleted_at IS NOT NULL)
             ORDER BY deleted_at DESC,id",
        )?;
        Ok(statement
            .query_map([], read_entry)?
            .collect::<std::result::Result<_, _>>()?)
    }

    pub fn restore_entry(&self, id: &str, expected_revision: i64) -> Result<()> {
        let changed = self.connection.execute(
            "UPDATE entries SET deleted_at=NULL, revision=revision+1 WHERE id=?1 AND revision=?2 AND deleted_at IS NOT NULL
             AND NOT EXISTS(SELECT 1 FROM experiment_logs l JOIN cvt_experiments c ON l.experiment_id=c.id WHERE l.project_id=entries.project_id AND c.deleted_at IS NOT NULL)",
            params![id,expected_revision],
        )?;
        if changed == 0 {
            return Err(StoreError::Conflict);
        }
        Ok(())
    }

    fn load_elements(&self, experiment: &mut CvtExperiment) -> Result<()> {
        let mut statement = self.connection.prepare("SELECT role,atomic_number FROM experiment_elements WHERE experiment_id=?1 ORDER BY atomic_number")?;
        let rows = statement.query_map([&experiment.id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u8>(1)?))
        })?;
        for row in rows {
            let (role, number) = row?;
            if element(number).is_none() {
                return Err(StoreError::Validation(
                    "Invalid saved element number.".into(),
                ));
            }
            match role.as_str() {
                "precursor" => {
                    experiment.details.precursor.elements.insert(number);
                }
                "product" => {
                    experiment.details.product.elements.insert(number);
                }
                _ => {
                    return Err(StoreError::Validation(
                        "Invalid saved material role.".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Prior project notes remain accessible without mixing them into experiments.
    pub fn legacy_projects(&self) -> Result<Vec<Project>> {
        let mut statement = self.connection.prepare("SELECT p.id,p.name,p.description,p.created_at FROM projects p WHERE NOT EXISTS(SELECT 1 FROM experiment_logs l WHERE l.project_id=p.id) ORDER BY p.created_at,p.id")?;
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
            "SELECT id, project_id, title, body, event_date, created_at, updated_at, revision FROM entries WHERE id = ?1 AND deleted_at IS NULL AND NOT EXISTS(SELECT 1 FROM experiment_logs l JOIN cvt_experiments c ON l.experiment_id=c.id WHERE l.project_id=entries.project_id AND c.deleted_at IS NOT NULL)",
            [id], read_entry,
        ).optional()?.ok_or(StoreError::NotFound)
    }

    /// Literal, Unicode-aware, case-insensitive substring search in the current
    /// project. Linear scanning is intentional for this small first milestone.
    pub fn entries(&self, project_id: &str, search: &str) -> Result<Vec<Entry>> {
        let mut statement = self.connection.prepare(
            "SELECT id, project_id, title, body, event_date, created_at, updated_at, revision
             FROM entries WHERE project_id = ?1 AND deleted_at IS NULL AND NOT EXISTS(SELECT 1 FROM experiment_logs l JOIN cvt_experiments c ON l.experiment_id=c.id WHERE l.project_id=entries.project_id AND c.deleted_at IS NOT NULL) ORDER BY event_date DESC, updated_at DESC, id",
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
        let transaction = self.connection.unchecked_transaction()?;
        self.validate_draft(draft)?;
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        self.connection.execute(
            "INSERT INTO entries (id, project_id, title, body, event_date, created_at, updated_at, revision)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 1)",
            params![id, draft.project_id, draft.title.trim(), draft.body, draft.event_date.to_string(), now],
        )?;
        let entry = self.entry(&id)?;
        transaction.commit()?;
        Ok(entry)
    }

    pub fn update_entry(
        &self,
        id: &str,
        expected_revision: i64,
        draft: &EntryDraft,
    ) -> Result<Entry> {
        let transaction = self.connection.unchecked_transaction()?;
        self.validate_draft(draft)?;
        let changed = self.connection.execute(
            "UPDATE entries SET title = ?1, body = ?2, event_date = ?3, updated_at = ?4, revision = revision + 1
             WHERE id = ?5 AND revision = ?6 AND project_id = ?7 AND deleted_at IS NULL",
            params![draft.title.trim(), draft.body, draft.event_date.to_string(),
                Utc::now().to_rfc3339(), id, expected_revision, draft.project_id],
        )?;
        if changed == 0 {
            return match self.entry(id) {
                Ok(_) => Err(StoreError::Conflict),
                Err(error) => Err(error),
            };
        }
        let entry = self.entry(id)?;
        transaction.commit()?;
        Ok(entry)
    }

    fn validate_draft(&self, draft: &EntryDraft) -> Result<()> {
        if draft.title.trim().is_empty() {
            return Err(StoreError::Validation(
                "Give the entry a title before saving.".into(),
            ));
        }
        let exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND NOT EXISTS(SELECT 1 FROM experiment_logs l JOIN cvt_experiments c ON l.experiment_id=c.id WHERE l.project_id=projects.id AND c.deleted_at IS NOT NULL))",
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

fn read_cvt_experiment(row: &Row<'_>) -> rusqlite::Result<CvtExperiment> {
    let conditions = CvtConditions::new(row.get(2)?, row.get(3)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(2, Type::Real, Box::new(error))
    })?;
    let heat: String = row.get(5)?;
    let heat_effect = HeatEffect::from_storage(&heat).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            Type::Text,
            Box::new(StoreError::Validation(
                "Unknown saved thermal character.".into(),
            )),
        )
    })?;
    Ok(CvtExperiment {
        id: row.get(0)?,
        name: row.get(1)?,
        conditions,
        created_at: parse_value(row, 4)?,
        log_project_id: row.get(9)?,
        details: ExperimentDetails {
            experiment_date: row
                .get::<_, Option<String>>(10)?
                .map(|value| value.parse::<NaiveDate>())
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(10, Type::Text, Box::new(error))
                })?,
            heat_effect,
            precursor: Material {
                text: row.get(6)?,
                ..Default::default()
            },
            product: Material {
                text: row.get(7)?,
                ..Default::default()
            },
            transport_agent: row.get(8)?,
        },
    })
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
