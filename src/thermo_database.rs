//! Installation-wide thermodynamic references. No values are bundled or inferred.
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::thermodynamics::{HeatModel, SpeciesRecord};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params, types::Type};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ThermoError {
    #[error("{0}")]
    Validation(String),
    #[error("Thermodynamic database version {0} needs a newer CrystalForge.")]
    UnsupportedVersion(i64),
    #[error("This is not an empty or recognised CrystalForge thermodynamic database.")]
    UnrecognisedDatabase,
    #[error("The requested thermodynamic record no longer exists.")]
    NotFound,
    #[error("This record changed in another app window. Reload it before editing.")]
    Conflict,
    #[error("Local file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Local thermodynamic database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}
pub type Result<T> = std::result::Result<T, ThermoError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermoSource {
    pub id: String,
    pub name: String,
    pub authors: String,
    pub year: i32,
    pub reference: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
}
impl Phase {
    pub const ALL: [Self; 3] = [Self::Solid, Self::Liquid, Self::Gas];
    pub fn code(self) -> &'static str {
        match self {
            Self::Solid => "s",
            Self::Liquid => "l",
            Self::Gas => "g",
        }
    }
    fn from_code(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|phase| phase.code() == value)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Solid => "Solid (s)",
            Self::Liquid => "Liquid (l)",
            Self::Gas => "Gas (g)",
        }
    }
}

/// Values are at an explicitly entered reference temperature. No Cp integration
/// or extrapolation is implied by storing this record.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermoEntryDraft {
    pub source_id: String,
    pub species: String,
    pub formula: String,
    pub phase: Phase,
    pub min_k: f64,
    pub max_k: f64,
    pub reference_k: f64,
    pub reference_pressure_bar: f64,
    /// Standard enthalpy of formation at the reference temperature, J/mol.
    pub enthalpy_j_mol: Option<f64>,
    /// Standard molar entropy at the reference temperature, J/(mol K).
    pub entropy_j_mol_k: Option<f64>,
    /// Standard molar heat capacity at the reference temperature, J/(mol K).
    pub heat_capacity_j_mol_k: Option<f64>,
    pub source_locator: String,
    pub comments: String,
}

impl ThermoEntryDraft {
    pub fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("minimum temperature", self.min_k),
            ("maximum temperature", self.max_k),
            ("reference temperature", self.reference_k),
            ("standard pressure", self.reference_pressure_bar),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(ThermoError::Validation(format!(
                    "Enter a finite, positive {name} in K."
                )));
            }
        }
        if self.min_k > self.max_k {
            return Err(ThermoError::Validation(
                "Minimum temperature must not exceed maximum temperature.".into(),
            ));
        }
        for (name, value) in [
            ("enthalpy", self.enthalpy_j_mol),
            ("entropy", self.entropy_j_mol_k),
            ("heat capacity", self.heat_capacity_j_mol_k),
        ] {
            if value.is_some_and(|value| !value.is_finite()) {
                return Err(ThermoError::Validation(format!(
                    "Enter a finite {name} value."
                )));
            }
        }
        if self.enthalpy_j_mol.is_none()
            && self.entropy_j_mol_k.is_none()
            && self.heat_capacity_j_mol_k.is_none()
        {
            return Err(ThermoError::Validation(
                "Enter at least one thermodynamic value.".into(),
            ));
        }
        for (label, value, limit) in [
            ("Species", &self.species, 160),
            ("Formula", &self.formula, 160),
            ("Source locator", &self.source_locator, 300),
        ] {
            if value.trim().is_empty() || value.chars().count() > limit {
                return Err(ThermoError::Validation(format!(
                    "{label} is required and must be at most {limit} characters."
                )));
            }
        }
        if self.comments.chars().count() > 4000 {
            return Err(ThermoError::Validation(
                "Comments must be at most 4000 characters.".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThermoEntry {
    pub id: String,
    pub draft: ThermoEntryDraft,
    pub imported_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub revision: i64,
}

impl ThermoEntry {
    /// Make an owned calculation input without treating point Cp as constant Cp.
    pub fn as_engine_record(&self, source: &ThermoSource) -> Result<SpeciesRecord> {
        if self.draft.source_id != source.id {
            return Err(ThermoError::Validation(
                "The entry and source database do not match.".into(),
            ));
        }
        Ok(SpeciesRecord {
            id: self.id.clone(),
            source_id: source.id.clone(),
            source_name: source.name.clone(),
            source_authors: source.authors.clone(),
            source_year: source.year,
            source_reference: source.reference.clone(),
            source_enabled: source.enabled,
            source_locator: self.draft.source_locator.clone(),
            species_name: self.draft.species.clone(),
            comments: self.draft.comments.clone(),
            imported_at: self.imported_at,
            updated_at: self.updated_at,
            revision: self.revision,
            verified_at: self.verified_at,
            formula: self.draft.formula.clone(),
            phase: self.draft.phase.code().into(),
            min_k: self.draft.min_k,
            max_k: self.draft.max_k,
            reference_k: self.draft.reference_k,
            reference_pressure_bar: self.draft.reference_pressure_bar,
            formation_enthalpy_j_mol: self.draft.enthalpy_j_mol,
            molar_entropy_j_mol_k: self.draft.entropy_j_mol_k,
            model: HeatModel::ReferenceOnly,
        })
    }
}

pub struct ThermoStore {
    connection: Connection,
    path: PathBuf,
}
impl ThermoStore {
    pub fn open(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root)?;
        Self::open_file(&root.join("thermodynamics.sqlite3"))
    }

    pub fn open_file(path: &Path) -> Result<Self> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > 1 {
            return Err(ThermoError::UnsupportedVersion(version));
        }
        if version == 0 {
            let count: i64 = connection.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )?;
            if count != 0 {
                return Err(ThermoError::UnrecognisedDatabase);
            }
            let tx = connection.transaction()?;
            tx.execute_batch(
                "CREATE TABLE thermo_sources (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL UNIQUE COLLATE NOCASE CHECK(length(trim(name))>0),
                authors TEXT NOT NULL,
                year INTEGER NOT NULL,
                reference TEXT NOT NULL,
                enabled INTEGER NOT NULL CHECK(enabled IN (0,1)),
                created_at TEXT NOT NULL
            );
            CREATE TABLE thermo_entries (
                id TEXT PRIMARY KEY NOT NULL,
                source_id TEXT NOT NULL REFERENCES thermo_sources(id),
                species TEXT NOT NULL CHECK(length(trim(species))>0),
                formula TEXT NOT NULL CHECK(length(trim(formula))>0),
                phase TEXT NOT NULL CHECK(phase IN ('s','l','g')),
                min_k REAL NOT NULL CHECK(min_k>0),
                max_k REAL NOT NULL CHECK(max_k>=min_k),
                reference_k REAL NOT NULL CHECK(reference_k>0),
                model_type TEXT NOT NULL CHECK(model_type='reference_values'),
                enthalpy_j_mol REAL,
                entropy_j_mol_k REAL,
                heat_capacity_j_mol_k REAL,
                source_locator TEXT NOT NULL CHECK(length(trim(source_locator))>0),
                comments TEXT NOT NULL,
                imported_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                verified_at TEXT,
                revision INTEGER NOT NULL CHECK(revision>=1),
                reference_pressure_bar REAL NOT NULL CHECK(reference_pressure_bar>0)
            );
            CREATE INDEX thermo_entries_source_species ON thermo_entries(source_id, formula, phase);
            CREATE TABLE thermo_preferences (
                id INTEGER PRIMARY KEY CHECK(id=1),
                primary_source_id TEXT NOT NULL REFERENCES thermo_sources(id),
                fallback_source_id TEXT REFERENCES thermo_sources(id)
            );",
            )?;
            tx.execute(
                "INSERT INTO thermo_sources VALUES ('knacke-1991',?1,?2,1991,?3,1,?4)",
                params![
                    "Knacke 1991",
                    "Knacke, Kubaschewski and Hesselmann",
                    "Thermochemical Properties of Inorganic Substances",
                    Utc::now().to_rfc3339()
                ],
            )?;
            tx.execute(
                "INSERT INTO thermo_preferences VALUES (1,'knacke-1991',NULL)",
                [],
            )?;
            tx.pragma_update(None, "user_version", 1)?;
            tx.commit()?;
        }
        connection.prepare(
            "SELECT id,name,authors,year,reference,enabled,created_at FROM thermo_sources LIMIT 0",
        )?;
        connection.prepare("SELECT id,source_id,species,formula,phase,min_k,max_k,reference_k,model_type,enthalpy_j_mol,entropy_j_mol_k,heat_capacity_j_mol_k,source_locator,comments,imported_at,updated_at,verified_at,revision,reference_pressure_bar FROM thermo_entries LIMIT 0")?;
        connection.prepare("SELECT primary_source_id,fallback_source_id FROM thermo_preferences WHERE id=1 LIMIT 0")?;
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
    pub fn sources(&self) -> Result<Vec<ThermoSource>> {
        let mut query=self.connection.prepare("SELECT id,name,authors,year,reference,enabled,created_at FROM thermo_sources ORDER BY created_at,id")?;
        Ok(query
            .query_map([], read_source)?
            .collect::<std::result::Result<_, _>>()?)
    }
    pub fn primary_source_id(&self) -> Result<String> {
        Ok(self.connection.query_row(
            "SELECT primary_source_id FROM thermo_preferences WHERE id=1",
            [],
            |row| row.get(0),
        )?)
    }
    pub fn set_primary_source(&self, id: &str) -> Result<()> {
        if !self
            .sources()?
            .iter()
            .any(|source| source.id == id && source.enabled)
        {
            return Err(ThermoError::NotFound);
        }
        self.connection.execute(
            "UPDATE thermo_preferences SET primary_source_id=?1,fallback_source_id=NULL WHERE id=1",
            [id],
        )?;
        Ok(())
    }
    pub fn create_source(
        &self,
        name: &str,
        authors: &str,
        year: i32,
        reference: &str,
    ) -> Result<ThermoSource> {
        let name = name.trim();
        let authors = authors.trim();
        let reference = reference.trim();
        if name.is_empty()
            || name.chars().count() > 160
            || authors.is_empty()
            || authors.chars().count() > 300
            || reference.is_empty()
            || reference.chars().count() > 500
            || !(1000..=2100).contains(&year)
        {
            return Err(ThermoError::Validation(
                "Enter a source name, authors, reference and valid year.".into(),
            ));
        }
        let source = ThermoSource {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            authors: authors.into(),
            year,
            reference: reference.into(),
            enabled: true,
            created_at: Utc::now(),
        };
        self.connection.execute(
            "INSERT INTO thermo_sources VALUES (?1,?2,?3,?4,?5,1,?6)",
            params![
                source.id,
                source.name,
                source.authors,
                source.year,
                source.reference,
                source.created_at.to_rfc3339()
            ],
        )?;
        Ok(source)
    }
    pub fn set_source_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        if !enabled && self.primary_source_id()? == id {
            return Err(ThermoError::Validation(
                "Choose another primary database before disabling this one.".into(),
            ));
        }
        if self.connection.execute(
            "UPDATE thermo_sources SET enabled=?1 WHERE id=?2",
            params![enabled, id],
        )? == 0
        {
            return Err(ThermoError::NotFound);
        }
        Ok(())
    }
    pub fn delete_empty_source(&self, id: &str) -> Result<()> {
        if id == "knacke-1991" || self.primary_source_id()? == id {
            return Err(ThermoError::Validation(
                "The primary or bundled source cannot be deleted.".into(),
            ));
        }
        if self.connection.execute("DELETE FROM thermo_sources WHERE id=?1 AND NOT EXISTS(SELECT 1 FROM thermo_entries WHERE source_id=?1)",[id])?==0 {
            return Err(ThermoError::Validation("Only an empty, non-primary database can be deleted.".into()));
        }
        Ok(())
    }
    pub fn entries(&self, source_id: &str) -> Result<Vec<ThermoEntry>> {
        let mut query=self.connection.prepare("SELECT id,source_id,species,formula,phase,min_k,max_k,reference_k,model_type,enthalpy_j_mol,entropy_j_mol_k,heat_capacity_j_mol_k,source_locator,comments,imported_at,updated_at,verified_at,revision,reference_pressure_bar FROM thermo_entries WHERE source_id=?1 ORDER BY formula,phase,min_k,id")?;
        Ok(query
            .query_map([source_id], read_entry)?
            .collect::<std::result::Result<_, _>>()?)
    }
    pub fn engine_records(&self, source_id: &str) -> Result<Vec<SpeciesRecord>> {
        let source = self
            .sources()?
            .into_iter()
            .find(|source| source.id == source_id)
            .ok_or(ThermoError::NotFound)?;
        self.entries(source_id)?
            .iter()
            .map(|entry| entry.as_engine_record(&source))
            .collect()
    }
    pub fn entry(&self, id: &str) -> Result<ThermoEntry> {
        self.connection.query_row("SELECT id,source_id,species,formula,phase,min_k,max_k,reference_k,model_type,enthalpy_j_mol,entropy_j_mol_k,heat_capacity_j_mol_k,source_locator,comments,imported_at,updated_at,verified_at,revision,reference_pressure_bar FROM thermo_entries WHERE id=?1",[id],read_entry).optional()?.ok_or(ThermoError::NotFound)
    }
    pub fn create_entry(&self, draft: &ThermoEntryDraft) -> Result<ThermoEntry> {
        draft.validate()?;
        if !self
            .sources()?
            .iter()
            .any(|source| source.id == draft.source_id && source.enabled)
        {
            return Err(ThermoError::Validation(
                "Choose an enabled source database.".into(),
            ));
        }
        let now = Utc::now();
        let entry = ThermoEntry {
            id: Uuid::new_v4().to_string(),
            draft: draft.clone(),
            imported_at: now,
            updated_at: now,
            verified_at: None,
            revision: 1,
        };
        self.connection.execute("INSERT INTO thermo_entries VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'reference_values',?9,?10,?11,?12,?13,?14,?15,NULL,1,?16)",params![entry.id,draft.source_id,draft.species.trim(),draft.formula.trim(),draft.phase.code(),draft.min_k,draft.max_k,draft.reference_k,draft.enthalpy_j_mol,draft.entropy_j_mol_k,draft.heat_capacity_j_mol_k,draft.source_locator.trim(),draft.comments,now.to_rfc3339(),now.to_rfc3339(),draft.reference_pressure_bar])?;
        self.entry(&entry.id)
    }
    pub fn update_entry(
        &self,
        id: &str,
        expected_revision: i64,
        draft: &ThermoEntryDraft,
    ) -> Result<ThermoEntry> {
        draft.validate()?;
        let saved = self.entry(id)?;
        if saved.draft.source_id != draft.source_id {
            return Err(ThermoError::Validation("Moving an entry between source databases is not supported. Create a new entry instead.".into()));
        }
        if saved.revision != expected_revision {
            return Err(ThermoError::Conflict);
        }
        let changed=self.connection.execute("UPDATE thermo_entries SET species=?1,formula=?2,phase=?3,min_k=?4,max_k=?5,reference_k=?6,enthalpy_j_mol=?7,entropy_j_mol_k=?8,heat_capacity_j_mol_k=?9,source_locator=?10,comments=?11,updated_at=?12,reference_pressure_bar=?13,verified_at=NULL,revision=revision+1 WHERE id=?14 AND revision=?15",params![draft.species.trim(),draft.formula.trim(),draft.phase.code(),draft.min_k,draft.max_k,draft.reference_k,draft.enthalpy_j_mol,draft.entropy_j_mol_k,draft.heat_capacity_j_mol_k,draft.source_locator.trim(),draft.comments,Utc::now().to_rfc3339(),draft.reference_pressure_bar,id,expected_revision])?;
        if changed == 0 {
            return Err(ThermoError::Conflict);
        }
        self.entry(id)
    }
    pub fn set_verified(
        &self,
        id: &str,
        expected_revision: i64,
        verified: bool,
    ) -> Result<ThermoEntry> {
        let saved = self.entry(id)?;
        if saved.revision != expected_revision {
            return Err(ThermoError::Conflict);
        }
        saved.draft.validate()?;
        let verified_at = verified.then(|| Utc::now().to_rfc3339());
        if self.connection.execute("UPDATE thermo_entries SET verified_at=?1,updated_at=?2,revision=revision+1 WHERE id=?3 AND revision=?4",params![verified_at,Utc::now().to_rfc3339(),id,expected_revision])?==0 {return Err(ThermoError::Conflict);}
        self.entry(id)
    }
}

fn converted<T: std::str::FromStr>(row: &Row<'_>, index: usize) -> rusqlite::Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let value: String = row.get(index)?;
    value.parse().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}
fn read_source(row: &Row<'_>) -> rusqlite::Result<ThermoSource> {
    Ok(ThermoSource {
        id: row.get(0)?,
        name: row.get(1)?,
        authors: row.get(2)?,
        year: row.get(3)?,
        reference: row.get(4)?,
        enabled: row.get(5)?,
        created_at: converted(row, 6)?,
    })
}
fn read_entry(row: &Row<'_>) -> rusqlite::Result<ThermoEntry> {
    let phase: String = row.get(4)?;
    let phase = Phase::from_code(&phase).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            Type::Text,
            Box::new(ThermoError::Validation("Unknown saved phase.".into())),
        )
    })?;
    let model: String = row.get(8)?;
    if model != "reference_values" {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            8,
            Type::Text,
            Box::new(ThermoError::Validation("Unknown saved model.".into())),
        ));
    }
    Ok(ThermoEntry {
        id: row.get(0)?,
        draft: ThermoEntryDraft {
            source_id: row.get(1)?,
            species: row.get(2)?,
            formula: row.get(3)?,
            phase,
            min_k: row.get(5)?,
            max_k: row.get(6)?,
            reference_k: row.get(7)?,
            reference_pressure_bar: row.get(18)?,
            enthalpy_j_mol: row.get(9)?,
            entropy_j_mol_k: row.get(10)?,
            heat_capacity_j_mol_k: row.get(11)?,
            source_locator: row.get(12)?,
            comments: row.get(13)?,
        },
        imported_at: converted(row, 14)?,
        updated_at: converted(row, 15)?,
        verified_at: row
            .get::<_, Option<String>>(16)?
            .map(|value| {
                value.parse().map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(16, Type::Text, Box::new(error))
                })
            })
            .transpose()?,
        revision: row.get(17)?,
    })
}
