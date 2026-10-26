#!/usr/bin/env bash
# Cloud development setup. Application use on Windows does not need this script.
set -euo pipefail
export CARGO_HOME=/workspace/.cargo
export RUSTUP_HOME=/workspace/.rustup
export PATH="$CARGO_HOME/bin:$PATH"

if [[ ! -x "$CARGO_HOME/bin/rustup" ]]; then
    installer=$(mktemp /tmp/crystal-forge-rustup.XXXXXX)
    trap 'rm -f "$installer"' EXIT
    curl --fail --show-error --silent --location https://sh.rustup.rs -o "$installer"
    sh "$installer" -y --profile minimal --default-toolchain 1.95.0 --no-modify-path
fi

rustup toolchain install 1.95.0 --profile minimal --component rustfmt --component clippy
cd "$(dirname "$0")/.."
cargo fetch --locked --target x86_64-unknown-linux-gnu
cargo test --locked --offline --no-default-features -j 4
cargo build --locked --offline -j 4
