# Thermodynamics architecture and prerequisites

Phase 2 adds a global local source catalog and manual species records. Knacke
1991 is present as metadata only; no thermodynamic values are bundled. The desktop
does not yet expose equilibrium calculations. Validation against the original source and
the Binnewies workbook must precede trustworthy calculated results.

Each record stores one phase, a valid temperature range, an explicit reference
temperature and standard pressure, ΔfH° in J/mol, S° and Cp° in J/(mol·K), a
page/table locator and comments. Records start unverified. A user marks them
verified only after reviewing the source; editing clears verification. The
initial model stores values at one reference temperature. It does not describe
temperature-dependent heat capacity or phase-transition energetics.
Aqueous records await an explicit concentration standard state in a later schema.

## Phase 3 engine

`src/thermodynamics.rs` is a pure Rust module, separate from SQLite and egui.
Rust keeps the existing offline Windows executable free of a Python runtime.
It uses signed rational stoichiometric coefficients and checks element balance
for supported formulas. Inputs are explicit record IDs from one selected source.
Each record must be verified, complete, within its validated temperature range,
and use the same standard pressure. The result retains an owned snapshot of
every species record and source citation.

The installed database's `reference_values` model is evaluated only at its
reference temperature. A distinct constant-Cp model is available to the engine
only when the assumption and range are explicitly supplied. Storing one Cp value
in the database never activates that model. The engine returns ΔH°, ΔS°, ΔG°,
ln K and log₁₀ K; K is omitted when it cannot be represented as a finite
positive number. The next phase will add automatic phase selection and
piecewise temperature behavior. Gas partial-pressure and electrode-potential
calculations require further activity and electron-count definitions.

## Storage boundary

Each profile owns its research database and Workbooks. Thermodynamic reference
databases belong to the local installation, not to a Workbook. Saved calculations
may reference a Workbook, but must retain immutable snapshots of every species
record and source used so later edits cannot change an old result.

## Proposed engine boundary

Keep the numerical engine independent of egui and SQLite. Its inputs should be
typed species records, reactions, phase mode, temperature grid, selected database,
gas standard state, and optional electron count. Its outputs should include
values with units, selected phase and record ID at each temperature, warnings,
and provenance. This can be tested as a library before UI integration. A
standalone Python engine is possible only if it can be bundled for offline Windows
use; the present executable does not require Python.

The initial model should distinguish species identity from phase-specific records
and valid temperature ranges. Reactions should store signed rational
stoichiometric coefficients, supporting e.g. 2/3 without binary rounding at
input. Automatic, fixed, and explicit phase selection must be separate modes.
Never extrapolate or fill a missing record silently. An unavailable value blocks
the affected result with a visible explanation.

The database manager should store source title, authors, year, page/table,
species/formula, phase, range, model, H/S/Cp values and units, verification status,
and verification timestamps. A primary source and explicit fallback policy should
be recorded with each calculation. No unreviewed PDF extraction is authoritative.

## Validation and export gates

Knacke 1991 is the intended initial source. Verified entries require the original
pages or a licensed data export. The existing Binnewies calculation workbook is
needed to compare approximately 3–5 reactions with phase and unit checks. The
user's Excel workbook is also required as the export template; copying its layout
cannot be validated from a description alone. The template must remain unchanged
when exporting. CSV can expose raw results in addition to the Excel format.

No numerical constants or sample results will be presented as Knacke values until
the records and independent comparisons are available.
