$ErrorActionPreference = 'Stop'
$package = 'target/release/package'
New-Item -ItemType Directory -Force $package | Out-Null
$dll = 'target/release/spellwheel.dll'
$dllHash = (Get-FileHash $dll -Algorithm SHA256).Hash.ToLowerInvariant()
Copy-Item 'assets/AtkinsonHyperlegible-OFL.txt' "$package/AtkinsonHyperlegible-OFL.txt" -Force
$diagnostics = $env:SETTINGS_DIAGNOSTICS -eq 'true'
$features = @()
if ($diagnostics) { $features = @('settings-diagnostics') }
$build = [ordered]@{
    repository = $env:GITHUB_REPOSITORY
    commit = $env:GITHUB_SHA
    run_id = $env:GITHUB_RUN_ID
    run_url = "https://github.com/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID"
    target = 'x86_64-pc-windows-msvc'
    features = @($features)
    rust_toolchain = 'nightly-2026-11-13'
    rustc = (rustc --version)
    cargo = (cargo --version)
    dll_sha256 = $dllHash
    dll_bytes = (Get-Item $dll).Length
    automated_checks = 'controller and settings panel/listener harness and diagnostic transport tests passed before build'
    in_game_tested_candidate = 'c1c27699566313d2b5d93bda8f0d3ac760da5579'
    runtime_evidence = 'User accepted candidate c1c2769 in Elden Ring App Ver. 1.17.2 / Calibrations Ver. 1.17.3. This native DLL is a separate build.'
}
$build | ConvertTo-Json -Depth 4 | Set-Content "$package/build-provenance.json" -Encoding utf8NoBOM
"$dllHash  spellwheel.dll" | Set-Content "$package/DLL-SHA256.txt" -Encoding utf8NoBOM
@'
Close Elden Ring before replacing the active spellwheel.dll with this file.
Preserve your existing spellwheel.toml and all other mod files.
Keep backup DLLs outside every mod-loading folder; only one Spell Wheel DLL should be active.
Left Shift + F10 opens Spell Wheel settings by default. Advanced mode opens the TOML editor.
This normal release has diagnostics compiled out. No .enable marker is needed.
If using Frame Generation, configure it before launching the game.
'@ | Set-Content "$package/INSTALL.txt" -Encoding utf8NoBOM
if ($diagnostics) {
    'Diagnostic feature build: create spellwheel-settings-diagnostics.enable beside the DLL and restart to enable its bounded trace.' |
        Set-Content "$package/INSTALL.txt" -Encoding utf8NoBOM
}
$archive = "$package/spellwheel-controller-input-windows-x64.zip"
Compress-Archive -Force -Path $dll,"$package/build-provenance.json","$package/DLL-SHA256.txt","$package/INSTALL.txt","$package/AtkinsonHyperlegible-OFL.txt" -DestinationPath $archive
$fontLicenseHash = (Get-FileHash "$package/AtkinsonHyperlegible-OFL.txt" -Algorithm SHA256).Hash.ToLowerInvariant()
$zipHash = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
@("$dllHash  spellwheel.dll", "$zipHash  spellwheel-controller-input-windows-x64.zip", "$fontLicenseHash  AtkinsonHyperlegible-OFL.txt") |
    Set-Content "$package/SHA256SUMS.txt" -Encoding utf8NoBOM
@"
The controller spell and quick-item wheels now distinguish short taps from holds. A tap cycles once on release. A hold opens the wheel without cycling.

- A changed wheel close plays the native equipment click once. Returning to the starting slot or cancelling is silent.
- Ranged precision aim and Telescope zoom retain native controls.
- The indicator snaps to the current slot and holds steady near slot boundaries. Both instant and on-close switching are supported.

Close Elden Ring and replace the active DLL. Keep your existing **spellwheel.toml**, and store backup DLLs outside mod-loading folders. If you use Frame Generation, configure it before launch. The ZIP includes the DLL, installation notes, font license, and build provenance.

In-game acceptance passed on Elden Ring App Ver. **1.17.2** / Calibrations Ver. **1.17.3** using candidate **c1c2769**. This release is built on GitHub Actions for Windows x64/MSVC with locked dependencies and the pinned nightly Rust toolchain.

Source: $env:GITHUB_SHA
Build: https://github.com/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID
"@ | Set-Content "$package/release-notes.md" -Encoding utf8NoBOM
