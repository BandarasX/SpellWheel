$ErrorActionPreference = 'Stop'
$package = 'target/release/package'
New-Item -ItemType Directory -Force $package | Out-Null
$dll = 'target/release/spellwheel.dll'
$dllHash = (Get-FileHash $dll -Algorithm SHA256).Hash.ToLowerInvariant()
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
    automated_checks = 'listener/shortcut harness and diagnostic transport tests passed before build'
    runtime_evidence = 'Settings fix source was user-tested in the affected setup; this newly built native DLL has not separately been game-tested.'
}
$build | ConvertTo-Json -Depth 4 | Set-Content "$package/build-provenance.json" -Encoding utf8NoBOM
"$dllHash  spellwheel.dll" | Set-Content "$package/DLL-SHA256.txt" -Encoding utf8NoBOM
@'
Close Elden Ring before replacing the active spellwheel.dll with this file.
Preserve your existing spellwheel.toml and all other mod files.
Keep backup DLLs outside every mod-loading folder; only one Spell Wheel DLL should be active.
Left Shift + F10 opens the existing TOML editor; a second deliberate chord closes it.
This normal release has diagnostics compiled out. No .enable marker is needed.
Keep HDR/display settings unchanged. Do not switch FG live as part of installing this fix.
'@ | Set-Content "$package/INSTALL.txt" -Encoding utf8NoBOM
if ($diagnostics) {
    'Diagnostic feature build: create spellwheel-settings-diagnostics.enable beside the DLL and restart to enable its bounded trace.' |
        Set-Content "$package/INSTALL.txt" -Encoding utf8NoBOM
}
$archive = "$package/spellwheel-settings-shortcut-fix-windows-x64.zip"
Compress-Archive -Force -Path $dll,"$package/build-provenance.json","$package/DLL-SHA256.txt","$package/INSTALL.txt" -DestinationPath $archive
$zipHash = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
@("$dllHash  spellwheel.dll", "$zipHash  spellwheel-settings-shortcut-fix-windows-x64.zip") |
    Set-Content "$package/SHA256SUMS.txt" -Encoding utf8NoBOM
@"
Fixes the existing TOML settings editor opening and immediately closing when nested presentation callbacks read the same shortcut release.

Each settings shortcut activation is now applied once per editor. Held keys do not repeat, focus loss discards an unfinished chord, and a second deliberate Left Shift + F10 chord still closes the editor. Original controller cycling and the HDR rendering path are preserved.

Source commit: $env:GITHUB_SHA
Build: https://github.com/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID
Target: native Windows x64/MSVC; pinned nightly-2026-11-13; locked dependencies; default features (diagnostics disabled).

The candidate source was tested by the user: the editor stayed open, closed normally, and controller spell/item cycling worked normally. Logs confirmed a 29.88-second open interval without duplicate activation despite nested Present calls. This newly built native release DLL has not separately been game-tested. No broader external-mod or HDR appearance validation is claimed.

Download spellwheel.dll directly, or the ZIP containing the DLL and provenance. Preserve your existing spellwheel.toml. SHA256SUMS.txt verifies both download formats.

The separate crash when switching FG live with QuestPath remains outside this fix. The graphical settings panel is not included.
"@ | Set-Content "$package/release-notes.md" -Encoding utf8NoBOM
