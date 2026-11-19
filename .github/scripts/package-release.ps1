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
    automated_checks = 'settings panel/listener harness and diagnostic transport tests passed before build'
    in_game_tested_candidate = '0bb5bab17b85e720d4d30aa39b48b85be5fb4d4a'
    runtime_evidence = 'All required checks passed in the final in-game checklist for candidate 0bb5bab. The optional blocked-Save check was skipped in-game. This native DLL is a separate build.'
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
$archive = "$package/spellwheel-settings-panel-windows-x64.zip"
Compress-Archive -Force -Path $dll,"$package/build-provenance.json","$package/DLL-SHA256.txt","$package/INSTALL.txt","$package/AtkinsonHyperlegible-OFL.txt" -DestinationPath $archive
$fontLicenseHash = (Get-FileHash "$package/AtkinsonHyperlegible-OFL.txt" -Algorithm SHA256).Hash.ToLowerInvariant()
$zipHash = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
@("$dllHash  spellwheel.dll", "$zipHash  spellwheel-settings-panel-windows-x64.zip", "$fontLicenseHash  AtkinsonHyperlegible-OFL.txt") |
    Set-Content "$package/SHA256SUMS.txt" -Encoding utf8NoBOM
@"
Spell Wheel HDR now has an in-game settings panel. Press **Left Shift + F10** by default to open it.

- **Appearance** includes wheel layout, selection effects, readability, tints, and an optional keyboard/mouse inner ring.
- **HDR** includes UI brightness and saturation. **Interface** lets you change the settings shortcut.
- The movable, resizable panel opens a non-interactive wheel preview. **Advanced mode** edits the same TOML draft as the graphical controls.
- **Save** applies supported input and visual settings immediately. **Discard** restores saved settings. Changes to mod paths, texture encoding, startup delay, and XInput startup policy take effect after you restart the game.

Close the game and replace the active DLL. Keep your existing **spellwheel.toml** and store backup DLLs outside mod-loading folders. If you use Frame Generation, configure it before launch. The ZIP includes the DLL, installation notes, font license, and build provenance. The SHA-256 file verifies the downloads.

The in-game checklist for candidate **0bb5bab** passed every required check. The optional blocked-Save check was skipped in-game and is covered by the automated harness. GitHub Actions reran the automated suite and built this Windows x64/MSVC DLL from the release source using locked dependencies and the pinned nightly toolchain.

Source: $env:GITHUB_SHA
Build: https://github.com/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID
"@ | Set-Content "$package/release-notes.md" -Encoding utf8NoBOM
