-- Additions shipped by Research Minion 0.2.0; apply after schema-v1.sql.
CREATE TABLE cvt_experiments (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
    source_c REAL NOT NULL CHECK(source_c >= -273.15 AND source_c <= 1.7976931348623157e308),
    sink_c REAL NOT NULL CHECK(sink_c >= -273.15 AND sink_c <= 1.7976931348623157e308),
    created_at TEXT NOT NULL
);
CREATE INDEX cvt_experiments_created ON cvt_experiments(created_at DESC,id);
PRAGMA user_version = 2;
