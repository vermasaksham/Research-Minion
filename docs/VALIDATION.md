# Validation record

Version 0.2 CVT increment, 25 October 2026. The development host is Debian Linux x64;
Windows execution is not claimed.

## Executed

| Check | Result |
| --- | --- |
| Pinned Rust 1.95.0 installation and locked dependency restore | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed |
| `cargo test --locked --offline` | 20 integration tests passed; no ignored/failed tests |
| `cargo test --locked --offline --no-default-features` | The same 20 integration tests passed without desktop features or network restore |
| `cargo build --locked` | Linux desktop build passed |
| `bash scripts/setup-cloud.sh` | Executed in 0.1; unchanged script, with its core-test and desktop-build steps rechecked in 0.2 |
| Windows GNU x64 cross-build | Passed; Windows UI subsystem confirmed in the executable |

Library, executable, and documentation test runners also report zero tests;
the meaningful assertions are 11 tests in `tests/logbook.rs` and 9 in
`tests/cvt.rs`. The logbook tests cover
on-disk close/reopen, exact notes persistence, stable identity, retrospective
dates, ordering, scoped/literal/Unicode search, invalid records and dates,
retained drafts, two-instance conflict detection, and refusal to alter newer or
unrelated databases.

The CVT tests cover signed/zero differences, decimal inputs, invalid/non-finite
temperatures, absolute zero, exact temperature round-trips, optional labels,
project-independent storage, common plot scales (including flat and extreme
ranges), display-only rounding, preservation of the version-1 logbook during
migration, and refusal to partially upgrade an incompatible version-1 file.

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
validation of the future Peak Explorer.

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
2. With networking disconnected, create a project and entry, save, restart,
   reopen, edit, search, and confirm the data folder shown in the interface.
   Confirm the initial CVT tab, add decimal/equal/reversed temperature pairs,
   inspect their chart/table values, restart, and confirm experiment persistence.
3. Exercise Save and continue, Discard changes, and Keep editing during project
   switches, entry switches, Reload saved, and native window closing. Make an
   invalid edit and verify that a failed save never closes or navigates away.
4. Open two instances against the same test folder; edit one record from both.
   Confirm the stale save reports a conflict and preserves the second draft.
5. Check 100%, 150%, and 200% display scaling, keyboard-only navigation, input
   methods, Unicode notes, screen-reader behaviour, and graphics-driver support.
6. On disposable data, close all instances and copy the complete data folder.
   Open the copied folder using `--data-dir` and confirm the records.

Attachments, autosave/crash recovery, readable export, recoverable deletion,
edit history, automatic backup/restore, Peak Explorer integration, and scientific
reference-value verification are unimplemented and untested. Revision numbers
are conflict counters, not retained historical versions.
