use crate::cvt::CvtConditions;
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeatEffect {
    #[default]
    Unspecified,
    Endothermic,
    Exothermic,
}

impl HeatEffect {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unspecified => "Not specified",
            Self::Endothermic => "Endothermic",
            Self::Exothermic => "Exothermic",
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unspecified => "unspecified",
            Self::Endothermic => "endothermic",
            Self::Exothermic => "exothermic",
        }
    }
    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "unspecified" => Some(Self::Unspecified),
            "endothermic" => Some(Self::Endothermic),
            "exothermic" => Some(Self::Exothermic),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Material {
    /// Original formula/name/description, preserved independently of suggestions.
    pub text: String,
    /// Atomic numbers explicitly confirmed by the researcher.
    pub elements: BTreeSet<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExperimentDetails {
    /// Researcher assignment, never inferred from the temperature difference.
    pub heat_effect: HeatEffect,
    pub precursor: Material,
    pub product: Material,
    pub transport_agent: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CvtExperiment {
    pub id: String,
    pub name: String,
    pub conditions: CvtConditions,
    pub details: ExperimentDetails,
    pub log_project_id: String,
    /// Documentation time, not a claim about when an experiment was performed.
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub body: String,
    /// The research event's calendar date; independent of documentation time.
    pub event_date: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Used to detect edits made by another application instance.
    pub revision: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryDraft {
    pub project_id: String,
    pub title: String,
    pub body: String,
    pub event_date: NaiveDate,
}

impl From<&Entry> for EntryDraft {
    fn from(entry: &Entry) -> Self {
        Self {
            project_id: entry.project_id.clone(),
            title: entry.title.clone(),
            body: entry.body.clone(),
            event_date: entry.event_date,
        }
    }
}
