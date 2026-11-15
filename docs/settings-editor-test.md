# Diagnostic build and manual game test

> Historical investigation/test record. See [the final ChatGPT handoff](spellwheel-settings-fix-chatgpt-handoff.md) for the completed publication status.

This build investigates the existing TOML editor; it does not fix the disappearance. Windows/game testing is outstanding. Do not change FG while the game is running: the handoff reports a crash in that operation with QuestPath enabled.

## Build or use the supplied bundle

The local bundle is `target/spellwheel-settings-diagnostics.zip`. It contains the diagnostic `spellwheel.dll`, `build-provenance.txt` (DLL/source hashes and exact tools), `source.patch`, these reports, and validation logs. No user TOML is bundled. The normal release output can be overwritten by another build, so use the DLL inside the bundle for this test.

On Windows x64 with MSVC initialized and nightly Rust, use the commands matching current CI:

```powershell
cargo test --locked --manifest-path tests/listener-harness/Cargo.toml
rustc --test --edition 2021 hudhook_fork/hudhook/src/settings_diagnostics.rs -o settings-diagnostics-tests.exe
./settings-diagnostics-tests.exe
$env:SPELLWHEEL_DIAGNOSTIC_REVISION = git rev-parse HEAD
cargo build --release --locked --features settings-diagnostics
Get-FileHash target/release/spellwheel.dll -Algorithm SHA256
```

Check each command's success before continuing. With uncommitted changes, append a worktree label and preserve the exact diff/hash; a HEAD label alone does not identify those changes. Alternatively, after these changes are committed on a review branch, manually run **Build Spell Wheel HDR** with `settings_diagnostics=true`. The expected CI artifact is `spellwheel-settings-diagnostics-windows-x64`, containing `spellwheel.dll` and its build record. This investigation did not push a branch or dispatch CI.

The existing Linux cross-build setup uses:

```bash
source /workspace/.spellwheel-tools/activate.sh
export CL=/FIstring.h
export SPELLWHEEL_DIAGNOSTIC_REVISION="$(git rev-parse HEAD)+settings-diagnostics-worktree"
cargo +nightly-2026-11-13 xwin build --release --locked --offline --target x86_64-pc-windows-msvc --features settings-diagnostics
```

Cross-build output: `target/x86_64-pc-windows-msvc/release/spellwheel.dll`. The local `CL` workaround addresses an existing dependency's missing C header under clang-cl; it is not a gameplay change or a required Windows CI setting.

## Prepare one short run

1. Exit the game. Save the current installed Spell Wheel DLL and TOML outside all loader/mod directories. Keep the current TOML in place; do not replace it with defaults or save edits during this test.
2. Replace only the active Spell Wheel DLL with the supplied diagnostic DLL. Keep backup DLLs outside loader/mod directories. Record the installed DLL's full path and SHA-256 and compare it with the bundle's build record.
3. Create an empty file named **`spellwheel-settings-diagnostics.enable` beside that DLL**. Ensure Windows has not appended `.txt`. Opt-in is checked only at startup; no TOML/debugging setting is needed.
4. Record the actual current game, ME3, QuestPath and ERSS versions; active ME3 profile and QuestPath entry state; FG enabled state and provider/mode; fullscreen/override settings, resolution and HDR settings. Preserve HDR/display/Spell Wheel values across comparisons. The November 7 screenshot is not proof of this run's FG state.
5. Establish the desired FG state before the test and restart the game. The user controls external settings; no live FG transition is part of this procedure. Collect EML's loaded-module log for this launch. Confirm the intended diagnostic DLL path/module identity and whether any other Spell Wheel module is loaded. A filename alone cannot establish identity.

If possible, collect the full module path/base inventory from PowerShell while the game is running, before returning focus to the game for the shortcut test:

```powershell
(Get-Process eldenring).Modules |
  Select-Object ModuleName, FileName, BaseAddress |
  Export-Csv -NoTypeInformation "$env:USERPROFILE\Desktop\spellwheel-module-inventory.csv"
```

If module enumeration is unavailable, provide the loader log and state that limitation; do not assume the module inventory was verified. The trace records its own module base, DLL path and build label for correlation. A second Spell Wheel identity would be new evidence; this task does not assume recurrence of the previously resolved duplicate-wheel incident.

## Same shortcut sequence for each condition

After reaching the same playable scene, return focus to the game and release all keys. Leave the controller idle for the capture.

1. Hold Left Shift, press F10, hold both for about half a second, release F10, then release Shift. Do this once. Do not press the shortcut again for at least 12 seconds. Note whether the editor never appears, flashes then disappears, or stays open. If it remains open, use one additional deliberate chord to test normal close, then wait another 12 seconds.
2. If the first capture is insufficient, repeat after releasing all keys, this time releasing Shift before F10. Record that order and the result separately. A new launch gives the cleanest capture if the event cap was reached.
3. Keep the game running in the playable scene for at least two seconds after observations so the game tick can flush queued events, then exit normally and copy the logs before another launch. Do not use HDR screenshot colors to judge output quality.

Start with the affected condition if reproducing it is straightforward. Use only the comparisons needed to distinguish it, restarting between conditions:

| QuestPath entry in ME3 | FG established before test | Purpose |
| --- | --- | --- |
| Enabled | On | Reported affected condition |
| Enabled | Off | Baseline |
| Disabled | On | Boundary control if needed |

These comparisons have not been run in Cloud. Do not infer which mod is responsible from the boundary alone. The separate crash when switching FG is not a reproduction step.

## Return the smallest useful evidence

- `spellwheel/settings-diagnostics-<pid>-<module-base>.log` beside the loaded DLL, plus the ordinary `spellwheel/spellwheel.log` from that same run. The normal log should report that settings diagnostics were enabled. If not, check the feature build, marker name/path and restart.
- EML loaded-module log and module inventory if available; installed diagnostic DLL path/hash and bundle build record.
- A short condition note with the actual environment in preparation step 4, key release order, number of deliberate chords, and visible result. Say whether the editor disappeared before any second shortcut.

The trace contains paths and the two shortcut keys' sampled bits, but no typed editor text. `mask=1` means Shift, `mask=2` means F10, `mask=3` means both, `mask=0` means neither sampled down. Those samples are focus gated. `focused` is a separate foreground-process snapshot.

Return a short capture with `dropped=0` and `capped=false` where possible. If either condition fails or the final batch is missing, absence of a toggle/draw event cannot establish what happened. Do not enable the existing broad `debugging` option for this task; it logs much more frequently.

To disable diagnostic file logging, remove the marker and restart. To end testing, exit the game and restore the saved DLL from outside the mod directories, keeping the user's TOML unchanged. No live hot-swap is needed.
