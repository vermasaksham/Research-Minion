# Windows preview downloads

Version **0.4.1** for Windows 10/11 x64:

- [Download the standalone executable](https://github.com/vermasaksham/Research-Minion/raw/refs/heads/main/downloads/Research-Minion-0.4.1-windows-x64.exe)
- [Download the portable ZIP and documentation](https://github.com/vermasaksham/Research-Minion/raw/refs/heads/main/downloads/Research-Minion-0.4.1-windows-x64-preview.zip)
- [SHA-256 checksums](SHA256SUMS.txt)

Run the EXE, or extract the ZIP and open `research-minion.exe`. No Rust installation
is required. Data stays separately in `%LOCALAPPDATA%\ResearchMinion`; keep that
folder when replacing the program. Schema 4 preserves older records and adds
recoverable deletion and optional experiment dates. Versions before 0.4 cannot open an
upgraded database. Earlier preview downloads remain for reference.

This update removes duplicate headings, improves the Conditions/Materials form,
and adds visible individual deletion buttons on note cards. Deleting a different
note preserves your current editor draft.

The initial Peak Explorer under Tools supports entered orthorhombic cells and
reflections, Bragg calculations, and saved analysis entries. The original tool's
Sb/Se EDX markers, CIF/reference catalog, and setting conversion are not included.

Built with Rust 1.95.0, `x86_64-pc-windows-gnu`, bundled SQLite and fonts. Forty-three
automated tests and Linux interface checks passed. Windows execution remains
unverified. Native MSVC CI is separate. These are portable previews, not installers.
Binaries are committed here because GitHub release-API access is unavailable.

See the [README](../README.md), [Peak scope](../docs/PEAK-EXPLORER.md), and
[validation record](../docs/VALIDATION.md).
