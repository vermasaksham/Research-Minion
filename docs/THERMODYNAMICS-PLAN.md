# Thermodynamics architecture and status

CrystalForge now has Workbooks, a global thermodynamic reference catalog,
manual and spreadsheet-derived candidate records, a pure Rust calculation
engine, phase-aware temperature series, a desktop calculator, saved Workbook
snapshots, CSV export and template-based Excel export. These run offline in the
Windows executable.

The reference catalog is local to the installation; it is not copied into
profiles or Workbooks. Each record stores source metadata, a cell/page locator,
formula, phase, temperature range, H/S anchor, standard pressure, heat capacity
model, revision and verification date. Entries start unverified. Editing clears
verification. Only verified records can be calculated. Knacke 1991 is metadata
only; the supplied spreadsheets are imported as separate worksheet sources.

`src/thermodynamics.rs` has no UI or database dependency. It uses signed
rational coefficients, checks element balance, selects one applicable record
per species at each temperature and stops on gaps, overlaps, missing values,
unverified records, mixed sources and extrapolation. Phase switches and record
changes are reported. The models are reference-point only, constant Cp and
`Cp(T) = a + bT + c/T² + dT²`. A point Cp value never activates a temperature
model by itself.

The engine calculates ΔH°, ΔS°, ΔG°, ln K and log₁₀ K. K is omitted if the
exponential is outside finite floating-point range. An ideal-gas partial
pressure calculation requires the activity of every other gas. A standard
electrode potential requires an explicit positive electron count z and uses
`E° = −ΔG°/(zF)` for the reaction as written. No aqueous standard state or
activity-coefficient model is implemented.

Every saved calculation belongs to the active profile and Workbook. It retains
the source IDs, citations, record revisions, values, phases, model and outputs
used at each point. The template export copies the user's `.xlsx` workbook,
preserves its original sheets/formulas/formatting and appends a results sheet.
The original template is never modified.

The supplied Knacke PDF exceeded the 32 MiB transfer limit. The user's later
instruction was to extract candidate values from the spreadsheets. The app
therefore provides a spreadsheet candidate import and individual review flow;
it does not claim automatic PDF extraction or independent Knacke verification.
The four worksheet checks and two significant discrepancies are documented in
[THERMODYNAMICS-VALIDATION.md](THERMODYNAMICS-VALIDATION.md).
