# Windows preview downloads

These are the version 0.2.0 preview binaries for Windows 10/11 x64. They are hosted
here so they can be downloaded through GitHub while release-API access is
unavailable in the development environment.

- [Download the standalone executable](https://github.com/vermasaksham/Research-Minion/raw/refs/heads/main/downloads/Research-Minion-0.2.0-windows-x64.exe)
- [Download the portable ZIP and documentation](https://github.com/vermasaksham/Research-Minion/raw/refs/heads/main/downloads/Research-Minion-0.2.0-windows-x64-preview.zip)
- [SHA-256 checksums](SHA256SUMS.txt)

Run the standalone executable after downloading, or extract the ZIP and open
`research-minion.exe`. The application stores data separately in
`%LOCALAPPDATA%\ResearchMinion`. Keep that data folder when replacing the program.

The binaries were built from this repository using Rust 1.95.0 and the
`x86_64-pc-windows-gnu` target with bundled SQLite and fonts. Twenty automated
tests and Linux interface checks passed. Windows execution is still unverified.
The native Windows CI workflow is separate from this cross-compiled preview.

The ZIP's bundled documentation reflects its build; the repository README adds
GitHub download instructions. These are preview files, not a complete installer.

See the [main README](../README.md) for functionality and setup, and the
[validation record](../docs/VALIDATION.md) for executed checks and limitations.
