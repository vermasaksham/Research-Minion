-- Schema shipped by the earlier 0.1.0 application, retained as an upgrade fixture.
CREATE TABLE projects (
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
PRAGMA user_version = 1;
