use chrono::{Local, NaiveDate};

use crate::{
    model::{Entry, EntryDraft},
    store::{Result, Store, StoreError},
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Fields {
    title: String,
    body: String,
    event_date: String,
}

/// Keeps the in-progress draft separate from saved data. A failed save never
/// clears the draft; revision checks protect against concurrent overwrites.
pub struct Editor {
    pub project_id: String,
    pub saved: Option<Entry>,
    pub title: String,
    pub body: String,
    pub event_date: String,
    baseline: Fields,
}

impl Editor {
    pub fn new(project_id: String) -> Self {
        let fields = Fields {
            title: String::new(),
            body: String::new(),
            event_date: Local::now().date_naive().to_string(),
        };
        Self {
            project_id,
            saved: None,
            title: fields.title.clone(),
            body: fields.body.clone(),
            event_date: fields.event_date.clone(),
            baseline: fields,
        }
    }

    pub fn from_entry(entry: Entry) -> Self {
        let fields = Fields {
            title: entry.title.clone(),
            body: entry.body.clone(),
            event_date: entry.event_date.to_string(),
        };
        Self {
            project_id: entry.project_id.clone(),
            saved: Some(entry),
            title: fields.title.clone(),
            body: fields.body.clone(),
            event_date: fields.event_date.clone(),
            baseline: fields,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.title != self.baseline.title
            || self.body != self.baseline.body
            || self.event_date != self.baseline.event_date
    }

    pub fn save(&mut self, store: &Store) -> Result<()> {
        let date_text = self.event_date.trim();
        let event_date = NaiveDate::parse_from_str(date_text, "%Y-%m-%d")
            .ok()
            .filter(|d| d.to_string() == date_text)
            .ok_or_else(|| {
                StoreError::Validation(
                    "Use a valid event date in YYYY-MM-DD format, for example 2026-10-25.".into(),
                )
            })?;
        let draft = EntryDraft {
            project_id: self.project_id.clone(),
            title: self.title.clone(),
            body: self.body.clone(),
            event_date,
        };
        let entry = match &self.saved {
            Some(saved) => store.update_entry(&saved.id, saved.revision, &draft)?,
            None => store.create_entry(&draft)?,
        };
        *self = Self::from_entry(entry);
        Ok(())
    }
}
