# Windows preview downloads

Version **0.6.0** for Windows 10/11 x64:

- [Download the standalone executable](https://github.com/vermasaksham/Research-Minion/raw/refs/heads/main/downloads/Research-Minion-0.6.0-windows-x64.exe)
- [Download the portable ZIP and documentation](https://github.com/vermasaksham/Research-Minion/raw/refs/heads/main/downloads/Research-Minion-0.6.0-windows-x64-preview.zip)
- [SHA-256 checksums](SHA256SUMS.txt)

Run the EXE, or extract the ZIP and open `research-minion.exe`. No Rust installation
is required. First launch asks for a full name and optional picture. The active
identity appears above Workspace. Profiles keep separate research databases;
the first profile owns earlier root-folder records, and later profiles start empty.

Research notes now offer Markdown source editing, a formatting toolbar and a
rendered Preview. Experiment details sit at the left of the header; navigation
controls sit at the upper right. The notes list and wider editor are below.
Individual deletion stays on note cards. Tools is called Scientific Utilities.

Keep the entire `%LOCALAPPDATA%\ResearchMinion` folder when replacing the app or
making a closed-app backup: profile catalog, pictures, all profile subfolders,
and any earlier root database. Older binaries do not manage profiles. Earlier
preview downloads remain for reference.

The initial Peak Explorer still uses entered orthorhombic cells and reflections.
Its original Sb/Se EDX/CIF catalogs are not included.

Built with Rust 1.95.0, `x86_64-pc-windows-gnu`, bundled SQLite/fonts/image decoding.
58 automated tests and Linux interface checks passed. Native Windows execution and
the Windows picture picker remain unverified. These are portable previews, not
installers. Binaries are committed here because release-API access is unavailable.

See the [README](../README.md), [design](../docs/DESIGN.md), and
[validation record](../docs/VALIDATION.md).
