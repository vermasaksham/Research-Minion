# Windows preview downloads

CrystalForge — crystal-growth experiments and scientific utilities.

Version **0.9.1** for Windows 10/11 x64:

- [Download the standalone executable](https://github.com/vermasaksham/Research-Minion/raw/refs/heads/main/downloads/CrystalForge-0.9.1-windows-x64.exe)
- [Download the portable ZIP and documentation](https://github.com/vermasaksham/Research-Minion/raw/refs/heads/main/downloads/CrystalForge-0.9.1-windows-x64-preview.zip)
- [SHA-256 checksums](SHA256SUMS.txt)

Run the EXE, or extract the ZIP and open `crystal-forge.exe`. No Rust installation
is required. First launch asks for a full name and optional picture. The active
identity appears above Workspace. Profiles keep separate research databases;
the first profile owns earlier root-folder records, and later profiles start empty.

Research notes now offer Markdown source editing, a formatting toolbar and a
rendered Preview. Experiment details sit at the left of the header; navigation
controls sit at the upper right. The notes list and wider editor are below.
Individual deletion stays on note cards. Tools is called Scientific Utilities.

Keep the entire `%LOCALAPPDATA%\CrystalForge` folder when replacing the app or
making a closed-app backup: profile catalog, pictures, all profile subfolders,
and any earlier root database. CrystalForge automatically continues to use an
existing `%LOCALAPPDATA%\ResearchMinion` folder when that is where prior data lives.
Older binaries do not manage profiles. Earlier preview downloads remain for reference.

Peak Explorer now provides separate EDX/XRD reference views. Its 46 selected
NIST experimental EDX markers cover Sb, Bi, Se, I, Cl, Br and S. Three CIFs
(Sb₂Se₃, Sb and trigonal Se) provide 48 selected XRD reflections. Source inspection,
Cu/Co/custom wavelengths, Pbnm/Pnma mapping and profile-local JSON snapshots work.
Iodine-containing XRD phases are not bundled. The ZIP includes retained source
files and a reproducible Gemmi validation script; Python is not needed to use the app.

Built with Rust 1.95.0, `x86_64-pc-windows-gnu`, bundled SQLite/fonts/image decoding.
74 automated tests and Linux interface checks passed. Native Windows execution and
the Windows picture picker remain unverified. These are portable previews, not
installers. Binaries are committed here because release-API access is unavailable.

See the [README](../README.md), [design](../docs/DESIGN.md), and
[validation record](../docs/VALIDATION.md).
