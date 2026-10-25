CREATE TABLE cvt_experiments (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
                    source_c REAL NOT NULL CHECK(source_c >= -273.15 AND source_c <= 1.7976931348623157e308),
                    sink_c REAL NOT NULL CHECK(sink_c >= -273.15 AND sink_c <= 1.7976931348623157e308),
                    created_at TEXT NOT NULL
                , heat_effect TEXT NOT NULL DEFAULT 'unspecified' CHECK(heat_effect IN ('unspecified','endothermic','exothermic')), precursor_text TEXT NOT NULL DEFAULT '', product_text TEXT NOT NULL DEFAULT '', transport_agent TEXT NOT NULL DEFAULT '');
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
CREATE TABLE experiment_elements (
                    experiment_id TEXT NOT NULL REFERENCES cvt_experiments(id),
                    role TEXT NOT NULL CHECK(role IN ('precursor','product')),
                    atomic_number INTEGER NOT NULL CHECK(atomic_number BETWEEN 1 AND 118),
                    PRIMARY KEY(experiment_id, role, atomic_number)
                );
CREATE TABLE experiment_logs (
                    experiment_id TEXT PRIMARY KEY NOT NULL REFERENCES cvt_experiments(id),
                    project_id TEXT UNIQUE NOT NULL REFERENCES projects(id)
                );
CREATE TABLE projects (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
                    description TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
CREATE INDEX cvt_experiments_created ON cvt_experiments(created_at DESC, id);
CREATE INDEX entries_project_date ON entries(project_id, event_date DESC, updated_at DESC);
PRAGMA user_version = 3;
