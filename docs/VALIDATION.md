# Validation record

Version 0.4 deletion/date/Peak increment, 25 October 2026. The development host is Debian Linux x64;
Windows execution is not claimed.

## Executed

| Check | Result |
| --- | --- |
| Pinned Rust 1.95.0 installation and locked dependency restore | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed |
| `cargo test --locked --offline` | 43 integration tests passed; no ignored/failed tests |
| `cargo test --locked --offline --no-default-features` | The same 43 integration tests passed without desktop features or network restore |
| `cargo build --locked` | Linux desktop build passed |
| `bash scripts/setup-cloud.sh` | Executed in 0.1; unchanged script, with its core-test and desktop-build steps rechecked in 0.4 |
| Windows GNU x64 cross-build | Passed; Windows UI subsystem confirmed in the executable |

Library, executable, and documentation test runners also report zero tests;
the meaningful assertions are 11 tests in `tests/logbook.rs` and 9 in
`tests/cvt.rs`, plus 5 each in `tests/chemistry.rs` and
`tests/experiment_records.rs`, 6 in `tests/deletion.rs`, and 7 in
`tests/peak.rs`. The logbook tests cover
on-disk close/reopen, exact notes persistence, stable identity, retrospective
dates, ordering, scoped/literal/Unicode search, invalid records and dates,
retained drafts, two-instance conflict detection, and refusal to alter newer or
unrelated databases.

The CVT tests cover signed/zero differences, decimal inputs, invalid/non-finite
temperatures, absolute zero, exact temperature round-trips, optional labels,
project-independent storage, common plot scales (including flat and extreme
ranges), display-only rounding, preservation of the version-1 logbook during
migration, and refusal to partially upgrade an incompatible version-1 file.

The additional tests cover all 118 element identities and table locations,
formula groups/hydrates/subscripts, Co versus CO, element-name prefixes and common
name parts, rejection of unknown matches, exact metadata round-trips, different
precursor/product sets, optional agent/manual-only input, independent persistent
logs, rollback of partially written records, and migration from schema 2 without
reassigning old project notes or inventing a thermal assignment.

## Checks added in 0.4

Deletion tests cover schema-3 migration without inferring old dates, preservation
of metadata/notes/links, experiment deletion and restoration after restart, hidden
logs rejecting new/edited notes, stale entry deletion conflicts, restore advancing
revision, individually deleted entries staying deleted after parent restore, and
whole-log deletion rejecting changed snapshots. An injected SQLite trigger failure
proves bulk deletion rolls back all rows rather than deleting part of a log.

Peak tests use analytic synthetic cells, not unverified material references:
λ=d=2 Å gives 2θ=60°; λ=2d gives 180°; λ>2d has no solution. Axial and mixed
orthorhombic reflections, sign invariance, axis permutation, wavelength dependence,
malformed/duplicate/bounded hkl inputs, non-finite/nonphysical values, and complete
saved report persistence are checked independently of the interface.

## Interface checks executed on Linux in 0.4

Used the real application under Xvfb with disposable data and independent Python
SQLite reads. Added a CVT record dated 2024-02-29 and confirmed its date appeared
in Recorded experiments and the log header. The log's dedicated Back control was
visible at 1000×650. Cancelled entry deletion, then confirmed it and restored it
through Recently deleted. Deleted an entire two-entry log, restored both entries,
then deleted/restored its parent experiment while preserving the same note IDs.

Opened Tools → Peak Explorer, entered a synthetic 3/4/12 Å cell, λ=1.5406 Å,
source/axis text and five explicit reflections including one inaccessible at that
wavelength. The plot and table showed positions; saved a multiline interpretation
into the selected experiment with a retrospective analysis date. Independent
SQLite reads confirmed the frozen inputs, hkl list, results, version, provenance,
interpretation and correct log link. Reopening checks retain these saved snapshots.
An invalid 2023-02-29 experiment date disabled Save and created no record.
Deletion of a dirty note displayed an explicit unsaved-edit warning; Cancel kept
the draft, and Back still prompted before discarding it. The screenshot data is
labelled synthetic and is never seeded into new workspaces.

## Interface checks executed on Linux in 0.3

Used the real desktop under Xvfb with disposable data and independent SQLite reads.

- The sidebar contains CVT Experiments and Tools, with no Logbook tab.
- Added an Endothermic experiment with source/sink temperatures, manual precursor
  text, a chemical-name product description and optional iodine-agent text.
- Formula/name input displayed Sb/Se suggestions. Add suggested selected the chips.
- Expanded the complete periodic table, switched between Precursor and Obtained
  material, and selected oxygen for the latter only. Saved rows confirmed that
  precursor/product sets remained separate and raw text was not rewritten.
- Clicked the experiment bar to open its log; created a dated multiline note and
  verified the experiment/log/entry relationship in SQLite.
- Restarted the app and reopened the record; its note and metadata persisted.
- Added a second Exothermic experiment with a blank transport agent and separate
  note. Switching between experiment bars kept the two note lists independent.
- New add forms reset scroll/collapse state; saved or retained drafts preserve
  their actual text and selections.
- At minimum window size, the form's Save/Cancel controls remained accessible.
  A draft containing only material text triggered normal close protection;
  Keep editing returned to that text without saving an extra experiment.
- Opened a schema-2 workspace containing an older CVT record and unlinked notes.
  Previous project logs opened the original note; clicking the old experiment
  opened its new empty independent log with unspecified metadata.

## CVT interface checks executed on Linux in 0.2

Used the real application under Xvfb with disposable local data. Confirmed:

- Launch opens CVT Experiments, including when the previous window closed on
  Tools. The sidebar switches between CVT Experiments, Tools, and Logbook.
- The + button opens the two-temperature form. Entering 650.5 and 600.25 °C
  previews 50.25 °C before saving; independently verified the SQLite record.
- Saved reversed set-points (550/600 °C) and equal set-points (625/625 °C).
  The chart/table showed −50 and 0 °C and used the same temperature scale.
- A below-absolute-zero source displayed validation and disabled Save.
- A native close request protected an unsaved experiment. Save and continue
  with invalid values kept the window/draft open; correcting and saving closed
  successfully. Reopening restored all four saved records.
  A final check caught overlapping form/confirmation layers after failed saves;
  only the confirmation is now rendered while navigating. Rechecked Keep editing
  with retained invalid input, correction with cleared feedback, Cancel, Discard
  on close (no extra record), and Save and continue (the expected new record).
- The temperature axis remained visible while scrolling the chart horizontally
  at the minimum window size. Vertical scrolling reached all four table rows.
- Opening the previous milestone's disposable logbook upgraded it to schema 2,
  preserved its two entries, and did not invent CVT records from note text.
- Unsaved logbook notes prompted before switching tabs. Keep editing cancelled
  navigation; Discard left saved notes unchanged and cleared the draft, avoiding
  repeated prompts on subsequent tab switches.
- Tools displayed Peak Explorer as planned, without an active scientific tool.

## Original logbook interface checks executed on Linux in 0.1

Ran the real desktop under Xvfb with software OpenGL, using a disposable data
folder outside the checkout. Actions were performed through the interface;
saved outcomes were checked independently using Python's SQLite reader.

- Created a project and entered its description.
- Saved an entry using Ctrl+S with a past event date; confirmed the database
  values, first revision, and notes.
- Opened the unsaved-change prompt when requesting a new entry.
- Chose Keep editing, then Save and continue; confirmed revision advancement
  and that the edited notes were saved before the new draft opened.
- Saved a second entry with multiple actual newline characters through the Save
  entry button; confirmed exact newline preservation in the saved body.
- Discarded an edit; confirmed the discarded text was absent from the database.
- Searched for a body term; visually confirmed only the matching entry remained.
- Sent a native window-close request with unsaved edits. The app kept running and
  prompted; Save and continue persisted the edit and the process then exited.
- Restarted the application against the same folder, edited the reopened record,
  and saved it; confirmed the same record and incremented revision.
- Entered an impossible date and attempted to save; confirmed visible validation
  and an unchanged saved record.
- Checked the minimum window size. Added editor scrolling after finding clipped
  metadata, then verified the ID and metadata could be reached by scrolling.

No real research records or seeded examples are included in the application.
The scientific-looking text used during smoke checks was disposable test input.
These checks do not replace Windows UI testing or constitute scientific
verification of material reference data.

## Cross-building on the prepared cloud machine

The optional MinGW compiler was downloaded using Debian's signed package indexes
and retained outside the repository under `/workspace/.mingw`. The Rust Windows
GNU target was installed with rustup. The following command builds the preview:

```sh
export CARGO_HOME=/workspace/.cargo
export RUSTUP_HOME=/workspace/.rustup
export PATH=/workspace/.cargo/bin:/workspace/.mingw/usr/bin:$PATH
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc-posix
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc-posix
cargo build --locked --offline --release --target x86_64-pc-windows-gnu -j 4
```

The native Windows/MSVC path in `scripts/build-windows.ps1` is a separate build
path and has not been executed here. `.github/workflows/check.yml` will run it on
a Windows runner when the workflow is triggered; no CI result is implied by
adding that file.

## Windows acceptance checks still required

1. Build/test with the MSVC toolchain and run the Windows packaging script, or
   extract the cross-compiled preview ZIP and launch its executable.
2. With networking disconnected, add an experiment and metadata, save, click its
   bar to create an entry, restart, reopen, edit, search, and confirm the data
   folder. Test decimal/equal/reversed temperatures, thermal assignments, both
   periodic-table targets, name/formula suggestions and the optional agent.
3. Exercise Save and continue, Discard changes, and Keep editing during project
   switches, entry switches, Reload saved, and native window closing. Make an
   invalid edit and verify that a failed save never closes or navigates away.
4. Open two instances against the same test folder; edit one record from both.
   Confirm the stale save reports a conflict and preserves the second draft.
5. Check 100%, 150%, and 200% display scaling, keyboard-only navigation, input
   methods, Unicode notes, screen-reader behaviour, and graphics-driver support.
6. Test experiment/whole-log/individual entry deletion, Cancel, and restoration,
   including individually deleted notes followed by parent experiment restore.
   Open Peak Explorer offline, enter a verified orthorhombic cell and reflections,
   save an analysis, and reopen it from the chosen experiment log.
7. On disposable data, close all instances and copy the complete data folder.
   Open the copied folder using `--data-dir` and confirm the records.

Attachments, autosave/crash recovery, readable export, permanent purge,
edit history, automatic backup/restore, original Peak Explorer reference-data
integration, and scientific reference-value verification remain unimplemented and untested. Revision numbers
are conflict counters, not retained historical versions.
