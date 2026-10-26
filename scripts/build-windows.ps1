# Run in PowerShell on Windows with Rust and the MSVC C++ build tools installed.
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') {
    throw 'Run this packaging script on Windows with the MSVC toolchain.'
}

Push-Location (Join-Path $PSScriptRoot '..')
try {
    $target = 'x86_64-pc-windows-msvc'
    rustup target add $target
    if ($LASTEXITCODE -ne 0) { throw 'Installing the Windows Rust target failed.' }
    cargo test --locked --target $target
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed; no package was produced.' }
    cargo build --locked --release --target $target
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed; no package was produced.' }
    $metadataJson = cargo metadata --locked --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Could not read package metadata.' }
    $metadata = $metadataJson | ConvertFrom-Json
    $version = ($metadata.packages | Where-Object name -eq 'research-minion').version
    $staging = 'target/packages/ResearchMinion'
    New-Item -ItemType Directory -Force -Path $staging | Out-Null
    Copy-Item "target/$target/release/research-minion.exe" $staging -Force
    Copy-Item 'README.md' $staging -Force
    New-Item -ItemType Directory -Force -Path "$staging/docs" | Out-Null
    Copy-Item 'docs/*.md' "$staging/docs" -Force
    New-Item -ItemType Directory -Force -Path "$staging/docs/images" | Out-Null
    Copy-Item 'docs/images/*.png' "$staging/docs/images" -Force
    Copy-Item 'docs/WINDOWS-PREVIEW.txt' "$staging/START-HERE.txt" -Force
    New-Item -ItemType Directory -Force -Path "$staging/data" | Out-Null
    Copy-Item 'data/peak' "$staging/data/peak" -Recurse -Force
    New-Item -ItemType Directory -Force -Path "$staging/scripts" | Out-Null
    Copy-Item 'scripts/build-peak-catalog.py' "$staging/scripts" -Force
    Copy-Item 'scripts/peak-validation-requirements.txt' "$staging/scripts" -Force
    $archive = "target/packages/Research-Minion-$version-windows-x64.zip"
    Compress-Archive -Path "$staging/*" -DestinationPath $archive -Force
    Write-Output "Created $archive"
}
finally {
    Pop-Location
}
