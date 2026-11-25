$ErrorActionPreference = 'Stop'
$package = 'target/release/package'
New-Item -ItemType Directory -Force $package | Out-Null
$dll = 'target/release/spellwheel.dll'
$dllHash = (Get-FileHash $dll -Algorithm SHA256).Hash.ToLowerInvariant()
Copy-Item 'assets/AtkinsonHyperlegible-OFL.txt' "$package/AtkinsonHyperlegible-OFL.txt" -Force
$handoff = 'spellwheel-wheel-details-handoff.md'
Copy-Item "docs/$handoff" "$package/$handoff" -Force
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
    automated_checks = 'controller, settings panel, listener and wheel-details harness plus diagnostic transport tests passed before build'
    in_game_tested_candidate = 'f6e20d8ff1955a39ce11a25fb311d3536893f866'
    runtime_evidence = 'The user confirmed the summon warning fix in the offline game after accepting the other wheel-details behavior during candidate testing. This native DLL is a separate build.'
}
$build | ConvertTo-Json -Depth 4 | Set-Content "$package/build-provenance.json" -Encoding utf8NoBOM
"$dllHash  spellwheel.dll" | Set-Content "$package/DLL-SHA256.txt" -Encoding utf8NoBOM
@'
Close Elden Ring before replacing the active spellwheel.dll with this file.
Preserve your existing spellwheel.toml and all other mod files.
Keep backup DLLs outside every mod-loading folder; only one Spell Wheel DLL should be active.
Left Shift + F10 opens Spell Wheel settings by default. Advanced mode opens the TOML editor.
Configure the card and item counts under Interface > Wheel details.
This normal release has diagnostics compiled out. No .enable marker is needed.
If using Frame Generation, configure it before launching the game.
'@ | Set-Content "$package/INSTALL.txt" -Encoding utf8NoBOM
if ($diagnostics) {
    'Diagnostic feature build: create spellwheel-settings-diagnostics.enable beside the DLL and restart to enable its bounded trace.' |
        Set-Content "$package/INSTALL.txt" -Encoding utf8NoBOM
}
$archiveName = 'spellwheel-wheel-details-windows-x64.zip'
$archive = "$package/$archiveName"
Compress-Archive -Force -Path $dll,"$package/build-provenance.json","$package/DLL-SHA256.txt","$package/INSTALL.txt","$package/AtkinsonHyperlegible-OFL.txt","$package/$handoff" -DestinationPath $archive
$fontLicenseHash = (Get-FileHash "$package/AtkinsonHyperlegible-OFL.txt" -Algorithm SHA256).Hash.ToLowerInvariant()
$zipHash = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
@("$dllHash  spellwheel.dll", "$zipHash  $archiveName", "$fontLicenseHash  AtkinsonHyperlegible-OFL.txt") |
    Set-Content "$package/SHA256SUMS.txt" -Encoding utf8NoBOM
@"
The highlighted spell or quick item now has a details card. Open **Interface → Wheel details** to customize it.

- Spells show initial cast FP with active modifiers, colored stat requirements, and an Insufficient FP warning when applicable.
- Quick items show costs, scaling, and availability warnings. Finite item counts appear at the bottom right of their icons, with an adjustable font size.
- Spirit Ash and Puppet warnings follow the current summon monument, and the active summon suppresses its own Cannot summon warning.
- The card has independent name, placement, transparency, and gold border settings.

Close Elden Ring and replace the active DLL. Keep your existing **spellwheel.toml** and store backup DLLs outside mod-loading folders. Set Frame Generation before launching. The ZIP includes installation notes, the font license, checksums, build provenance, and the handoff.

The user confirmed the final summon fix in-game on the offline candidate **f6e20d8**. GitHub Actions built this separate Windows x64/MSVC DLL from the release source with locked dependencies and pinned nightly Rust.

[Release source](https://github.com/$env:GITHUB_REPOSITORY/commit/$env:GITHUB_SHA) · [Windows build and tests](https://github.com/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID) · [Handoff](https://github.com/$env:GITHUB_REPOSITORY/blob/$env:GITHUB_SHA/docs/$handoff)
"@ | Set-Content "$package/release-notes.md" -Encoding utf8NoBOM
