# Foundation and decisions

## Confirmed scope

Research Minion is a Windows desktop application that works offline, centred on
a research logbook. Scientific tools will be defined incrementally; the existing
Sb₂Se₃ Peak Explorer is the first intended tool. AI is not a requirement.

The implementation language is Rust, following the user's direction. The first
milestone covers projects and entries, local persistence, editing, and search.
Other proposed capabilities are a roadmap, not commitments already implemented.

The next confirmed increment adds a left sidebar with CVT Experiments, Tools,
and Logbook, opens on Experiments, visualises source/sink temperatures, and
calculates their difference when adding a new experiment.

## Architecture

- `model.rs`: saved project/entry/CVT records and typed entry drafts.
- `cvt.rs`: validated temperatures, signed difference, and common chart scale.
- `cvt_view.rs`: experiment overview, temperature form, and Tools page.
- `store.rs`: SQLite access, schema initialisation, validation and revision checks.
- `editor.rs`: unsaved editor state and date validation, independent of the GUI.
- `app.rs`: the egui desktop interface and navigation/save prompts.
- `main.rs`: local data-folder selection and desktop startup, including visible
  error reporting if a workspace cannot be opened.
- `tests/logbook.rs`: persistence, search, validation and conflict tests using
  disposable on-disk databases.
- `tests/cvt.rs`: temperatures, persistence, plotting ranges and schema upgrade
  from the retained version 1 fixture.

The native interface uses egui/eframe with its OpenGL renderer. This avoids a web
runtime and allows the same UI to run on Linux for development. Compared with
WPF, Rust offers shared native scientific/storage code and cross-platform testing,
but egui's controls look less like standard Windows controls and require further
Windows accessibility, screen-reader, IME, and high-DPI assessment. This prototype
does not claim those assessments are complete. A Tauri/WebView2 shell would reuse
web interfaces more directly, at the cost of managing a Windows web runtime and
its offline distribution.

There is one local SQLite connection per instance, a five-second busy timeout,
foreign keys enabled, WAL journalling, and FULL synchronous writes. SQLite is
bundled through `rusqlite`; no database service or separately installed SQLite is
needed. Schema changes belong in versioned migrations; unknown newer versions
are rejected instead of downgraded. Initialisation is transactional. The first
schemas have no destructive operations.

## Data model, version 2

| Record | Stored fields |
| --- | --- |
| Project | UUID, name, description, UTC creation time |
| Entry | UUID, project UUID, title, plain-text body, event date, UTC creation/edit times, revision |
| CVT experiment | UUID, name, source temperature (°C), sink temperature (°C), UTC recording time |

Schema version 2 adds the CVT table in a transaction; projects and entries retain
their existing IDs, contents and timestamps. A regression fixture verifies the
upgrade from the schema shipped in 0.1.0. CVT experiments do not yet require a
project or create a logbook entry automatically.

### CVT overview

The default opening tab is CVT Experiments on every launch. Each saved experiment
has a source point and sink point on the same Celsius scale as all other records.
The x axis only places these pairs side by side, with Source left and Sink right.
It is neither a length scale nor a timeline; the joining lines are schematic.
The temperature axis remains fixed when scrolling through more experiments.
A text table lists the same values, most recently recorded first.

The form requires only two finite temperatures, each at least −273.15 °C, and
previews **ΔT = Tsource − Tsink**. A cooler source gives a negative difference;
equal temperatures give zero. No assumptions about transport direction, chemical
equilibrium, furnace length, or actual spatial gradients are made. The optional
name gets an ID-based label when blank. Inputs are stored as IEEE 754 doubles,
with explicit Celsius column names. ΔT is derived, avoiding inconsistent copies.
Display rounds to six decimals or scientific notation without changing storage.

Experiments can be added and reopened in this increment; editing/deletion,
performed dates and links to projects/samples/entries await further requirements.
The saved timestamp records documentation time, not when an experiment ran.
Tools contains a planned Peak Explorer tile, without invented calculations or
reference values. New experiment drafts are protected on normal window closing;
unsaved logbook edits are protected when switching tabs.

The event date is a calendar date, without a time zone. Creation/edit times are
UTC instants. Retrospective documentation changes the event date without changing
the entry's creation time. UUIDs are assigned once and persist across edits.

Units and scientific notation in notes are preserved as text; the application
does not interpret them or make scientific claims. Editing trims the title's
outer whitespace but preserves notes exactly. Saving uses parameterised SQL.
Updates include the last loaded revision in the SQL condition. A stale update
fails and retains the in-memory draft so it can be copied or reconciled.

Search is a literal substring match over titles and notes in the selected
project, using Unicode lowercasing. It is not fuzzy search, canonical Unicode
normalisation, or full Unicode case folding. It scans that project's entries;
indexed full-text search can be introduced when realistic data volumes warrant
it. Entry bodies are plain text, not Markdown rendering or rich text.

Only explicit saves are durable. Unsaved changes are compared with the last
loaded/saved values. Navigation and normal window-close requests offer Save and
continue, Discard changes, or Keep editing. A failed save leaves the editor and
prompt open. This is not crash recovery or autosave.

## Proposed next increments

1. Review the first workflow on Windows, then decide the next data-protection
   increment: autosave/recovery, edit history, and versioned backup/restore.
2. Choose readable export formats and attachment ownership rules. A backup must
   include both the database and all owned files; a database-only export is not
   a complete future workspace backup.
3. Extend CVT records and define samples and tasks using real examples. Add linked tables
   and migrations instead of encoding permanent relationships in note text.
4. Inspect and integrate the existing Peak Explorer, then connect saved analyses
   to the logbook with stable IDs and versioned scientific provenance.
5. Validate Windows packaging, DPI behaviour, accessibility, and distribution.

No new scientific tools have been invented or scaffolded.

## Peak Explorer integration assessment

Its source and connector access are not available in this checkout. The previous
session's reported 10 tests, 48 reciprocal-metric checks, and CIF hash checks are
handover information, not results from this implementation.

Before implementation, retrieve the actual `dist/`, `research/`, `tests/`, README,
and validation files. Preserve source CIFs and reference data verbatim, record
their hashes, and rerun the existing tests and independent metric validator.

Two integration paths should be compared against that source:

| Option | Benefits | Costs |
| --- | --- | --- |
| Local web view module | Reuses the existing HTML/JavaScript calculations and UI; least scientific rewrite | Requires a carefully bundled offline Windows web runtime and a narrow native bridge |
| Rust calculations and egui interface | One native UI/runtime; shared typed analysis records | Reimplements UI and calculations; requires parity tests for every retained result and convention |

Do not choose a scientific rewrite just to match the shell. A retained local web
module may be preferable after inspecting the actual source. Neither path is
implemented yet. The private hosted URL must not become a runtime dependency.

Any future Save analysis to logbook record should preserve input selections,
wavelength and its convention, explicit Pbnm/Pnma setting, units, calculation
version, reference-data version/hash, source identifiers, results, timestamp, and
user interpretation. Links to samples/experiments await those records' designs.

Retain the distinctions between EDX energy (keV) and XRD position (2θ degrees),
the selected-reflections limitation, and the absence of intensity, phase-fraction,
or automatic phase-identification claims. Reference values require their original
provenance and uncertainty notation; none are fabricated in this milestone.
