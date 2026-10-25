use crate::cvt::CvtConditions;
use chrono::{DateTime, NaiveDate, Utc};

#[derive(Clone, Debug, PartialEq)]
pub struct CvtExperiment {
    pub id: String,
    pub name: String,
    pub conditions: CvtConditions,
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
