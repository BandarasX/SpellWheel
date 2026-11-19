# ChatGPT handoff for the Spell Wheel settings panel release

Published 19 November 2026 on the `BandarasX/SpellWheel` fork. [GitHub release](https://github.com/BandarasX/SpellWheel/releases/tag/settings-panel-56c8ed2) · [Native Windows build](https://github.com/BandarasX/SpellWheel/actions/runs/44562619176)

The release source is [`56c8ed24145c7c9002946047781af91fbf8a5447`](https://github.com/BandarasX/SpellWheel/commit/56c8ed24145c7c9002946047781af91fbf8a5447). The settings, renderer, input, dependencies and test sources match the locally built and in-game-tested candidate `0bb5bab17b85e720d4d30aa39b48b85be5fb4d4a`. The release packaging changes provide the panel release notes, install directions, provenance, and the Atkinson Hyperlegible font license. The published DLL is a separate native Windows x64/MSVC build. GitHub Actions passed the 43 panel/listener tests and 3 diagnostic transport tests before building it.

## Acceptance and scope

- `d8467e8` passed 45 of 47 required checks. The panel obscured the preview during the remaining two checks. Both checks passed on the next candidate, `51e2a80`, with the compact, movable panel, gold colors, Atkinson Hyperlegible, and percentage sliders. The optional blocked-Save check was skipped in that in-game run.
- `e33df59` passed all six checks of the half-point sliders, exact-entry fields, darker Advanced editor, and automatic non-interactive preview. The default preview uses quick items and falls back to spells when needed.
- `9c701da` passed all six checks of the optional keyboard/mouse inner ring and the controller-style automatic preview. A preview of an already visible wheel retains that wheel and its input style.
- `0bb5bab` passed all nine required checks in the final checklist. The live Advanced values, controller delay, debug overlay, Discard, restart notice, shortcut, and wheel input paths were verified in-game. The optional blocked-Save check was skipped in-game. The automated suite verifies that the runtime settings stay intact after a failed Save.
- The settings panel uses keyboard and mouse. The in-game tests did not validate a live Frame Generation switch. Configure Frame Generation before launch.

See [the settings guide](spellwheel-panel.md) for the five restart-only settings, rollback, and configuration details. Keep `spellwheel.toml` when replacing the active DLL. The release ZIP contains the DLL, install instructions, build manifest, checksum, and font license.

## Verified public downloads

| Asset | SHA-256 |
| --- | --- |
| `spellwheel.dll` | `b564a8bbf477036a485cffd8a931d8b5a06f3dd89523f171d07e82f479b3022d` |
| `spellwheel-settings-panel-windows-x64.zip` | `bef5b0a64de31e3d764b53455bb9e84d87ded5b8d7ff391c8edeebceabbba014` |
| `AtkinsonHyperlegible-OFL.txt` | `b1bc2009cb315a5c6d1758bb28eb930c9f986a89ad9e6fe5661b6710f492ffc4` |

Downloaded all five public assets and verified the checksums, ZIP contents, Windows x64 DLL header, and matching provenance. The DLL inside the ZIP is byte-identical to the direct download. The release tag points to the build commit and GitHub lists it as the latest release.

## Historical implementation handoff for the first candidate

The snapshot below documents the first local candidate. Use the release record above and the settings guide for the current status and behavior.

Prepared 15 November 2026. Self-contained implementation and candidate-build report. This does not claim Elden Ring or native Windows acceptance.

## Delivered state and review boundary

The approved native settings panel is implemented on local branch **`implement/approved-settings-panel`**. Implementation commit: **`d8467e89ecc3a225f33eb647011f50e514a025b3`**, based on the clean configured checkout **`a7f6344eedad8dbaea04d41403df05f557a6bc57`**. A documentation-only follow-up records this report; the candidate DLL is built from the implementation commit above. No existing work was reset.

Repository/fork: https://github.com/BandarasX/SpellWheel. Upstream https://github.com/notahackusator/SpellWheel was not modified. The branch/commits are local: **no push, PR, merge, master update, release publication, installed DLL/config replacement or external-mod change occurred**. The existing master-push publishing workflow was not triggered. Its test step now describes the expanded existing harness; publication behavior was left intact.

The task specification is preserved in [spellwheel-panel-approved-spec.md](spellwheel-panel-approved-spec.md). [spellwheel-panel.md](spellwheel-panel.md) documents behavior, keys, defaults, validation domains, reload/restart requirements and game acceptance. Review with `git diff a7f6344..d8467e8` or the candidate's `source.patch`.

## Implemented behavior

- One responsive Dear ImGui settings window, Appearance/HDR/Interface tabs, independent non-nested collapsing sections, visible active mode and binding, dirty/new-file/read-error status, scrollable content/error feedback, and persistent footer. Main sections start expanded. Section/tab state survives mode changes; drafts are not recreated. Keyboard focus remains on a collapsed header.
- All approved Appearance controls, explicit multiplier/percentage conversions, Pretty-only disable/annotation without value loss, HDR enabled/brightness/saturation through the existing working shader-constant path, and Interface F1–F12 with one of None/Left Shift/Left Ctrl/Left Alt.
- Graphical and Advanced share one TOML document. `toml_edit` changes managed values while retaining comments, key ordering, unknown tables/keys, mod paths, unmanaged fields and untouched omitted keys. Advanced uses the original native ImGui multiline editing widget with standard selection/scrolling and persistent precise syntax/schema/domain errors. Returning from invalid Advanced text is blocked without dropping text.
- Separate saved text/settings, current draft, last valid preview and effective runtime. Only approved visual fields preview; invalid text preserves the last valid preview. A wheel already open when settings opens can render with frozen selection; no sample-wheel studio or gameplay action is created for preview. Geometry caches respond to layout changes.
- Save validates, writes/syncs a unique same-directory temporary file, checks the disk baseline again, closes the handle and replaces through Windows-compatible `std::fs::rename`. It returns an actual Result. Failure retains original file, saved binding/state and draft. Success clears dirty state and keeps the panel open. Discard reverses Advanced edits and staged resets. Dirty close has Save/Discard/Continue choices; invalid drafts cannot be saved.
- Disk refresh cannot replace a draft/preview during editing. Disk changes are detected even for a clean open draft; Save requires explicit reload/overwrite choice. Explicit Reload disk never saves. Unreadable files keep runtime values and block editing/Save until successful explicit reload.
- Reset stages Current tab by default in Graphical or explicit All panel settings in Advanced. Cancel changes nothing. Confirm changes only managed keys, warns about all-panel HDR/hotkey effects, remains dirty/reversible and never writes. Invalid TOML blocks reset until repaired or explicitly discarded. The customized controller delay 0.25 and mod paths are preserved.
- Existing settings shortcut generation/per-editor consumption is preserved and generalized. All supported chords have intentional release activation, no held-key repeats, either release order and focus/startup rearming. None rejects extra Shift/Ctrl/Alt; modifier sides are strict. Reconfiguration waits for old/new key release. An epoch's starting generation drops old queued events while retaining valid new-binding events when rendering is delayed. Failed Save never installs a new binding. The saved Left Alt + F4 chord has a targeted native system-key filter rather than globally blocking window close.
- Menu input ownership extends existing window/DirectInput/raw-mouse/XInput infrastructure and gates wheel selection/cycling hooks. XInput state is captured before game suppression. Outside settings, original cycling logic is retained. The game continues running; no pause, save, networking, anti-cheat, game-speed or external presentation settings are changed.

Implementation map: `src/settings/panel_model.rs` owns the document, validation, persistence and pure runtime model; `display_settings.rs` owns widgets/modals; `mod.rs` coordinates the cache; `values.rs` contains the moved compatibility settings/schema. `settings_binding.rs`/`settings_shortcut.rs` own binding validation/generations. `input_policy.rs`, the existing input hooks and a small hudhook synchronous window-input predicate implement ownership. Existing HDR shader/backend rendering equations were not changed.

## Defaults, compatibility and staging

New keys are `settings_function_key = "F10"` and `settings_modifier = "left_shift"`. Allowed values are F1–F12 and `none`, `left_shift`, `left_ctrl`, `left_alt`. A bad startup binding, including wrong types, logs a diagnostic and uses **Left Shift + F10 in memory only**, preserving unrelated values and the original file for Advanced repair. Recovery while closed: back up the TOML, correct/remove those two fields, relaunch.

`project_defaults()` supplies the approved panel/new-file values: Pretty; icon 0.15; radius 0.3; enlargement/time 0.15/0.15 seconds; names show; font 1; shadows true; all three RGBA tints [1,1,1,1]; HDR **true / 400 nits / saturation 1**; Left Shift + F10. The new-config template documents these. Compatibility fallbacks remain HDR **false / 200 nits** for omitted keys in an existing document. Explicit user values survive, and opening never rewrites a file or forces 400 nits.

Only managed visual values preview/apply live. Hotkeys apply from a successful Save or explicit acceptance of persisted disk settings. All unmanaged Advanced changes, including controller delay, wheel input, selection thresholds, timing, hooks, debugging, encoding and paths, require **Save and game restart**. Their live startup values remain unchanged through preview, Save and periodic rereads. Advanced Reload icons is disabled for unsaved/invalid/conflicting drafts or runtime path/encoding differences; it remains an explicit icon-resource action.

All-panel reset excludes controller/technical fields, mod paths and unknown keys. Numeric inputs never silently clamp or rewrite out-of-domain text. Exact domains and rationale are in the usage document: conservative geometry/text budgets, positive selector clearance (`radius > icon + 0.01`), finite calculations through a 16384-pixel dimension, HDR 50–2000 nits / saturation 0–2 matching the existing backend, and RGBA 0–1. Existing supported values beyond ordinary UI suggestions remain intact. Legacy case-insensitive style/name values remain accepted.

## Actual automated results

| Check | Result |
| --- | --- |
| Pinned host listener/panel harness | **38 passed, 0 failed** |
| Existing diagnostic transport suite | **3 passed, 0 failed**, source unchanged |
| Windows x64/MSVC all-features **release** check | Passed |
| Windows x64/MSVC normal/default-feature **release DLL** cross-build | Passed |
| Formatting of new/reworked modules and harness | Passed with pinned rustfmt |
| `git diff --check` | Passed |
| Candidate PE inspection | AMD64 PE32+ DLL; diagnostic-enable/activation trace strings absent |
| Native Windows CI/game execution | **Not run** |

The original **13 shortcut/listener tests** remain and pass. Expanded coverage includes all **48** supported key/modifier bindings, both release orders, repeated/held input, startup/focus loss/regain, wrong-side/extra modifiers, nested draws, queued generations, delayed rendering across Save/rebind, and failed Save keeping the original binding. Document tests cover comments/Unicode/unknown tables/unmanaged values/omitted keys, shared mode/tab round trips, schema/domain failures, atomic save/error/conflict paths, Discard/dirty close, scoped/all resets, project-vs-compatibility defaults and runtime preview isolation.

The harness also instantiates the **actual production ImGui panel**, using the repository's Latin font at its production base size/scaling. It clicks reset/modal/mode actions, verifies invalid-text retention and save-error recovery, and checks footer geometry at **3440×1440, 1280×720, 800×600 and 640×480** in both modes. Left-arrow collapse retains header focus, independently expanded sections persist, and unreadable-open recovery retains runtime values. These are headless widget/layout tests, not DX12/HDR screenshots, game-input tests or proof of physical display readability.

Exact commands are in `spellwheel-panel.md`; the artifact directory includes full check/test/build logs. The pinned toolchain is **nightly-2026-11-13**, rustc **1.102.0-nightly (8eb778d04 2026-11-12)**, cargo-xwin **0.20.2**, clang-cl **19.1.7**, cached Windows SDK/MSVC x64 target, and the existing `CL=/FIstring.h` libudis86 C-header workaround. Builds were locked/offline. Existing compiler-family, unused/deprecated-code warnings remain non-fatal; no transport-source workaround or dependency-source patch was added. Root Cargo.lock SHA-256: `34651e267d5722ce5131f2833b66d868ad76297bdde11437b39fe2828eaf03f8`.

## Candidate and provenance

Artifact directory: **`/workspace/SpellWheel/artifacts/approved-panel-d8467e8/`**.

| Item | Value |
| --- | --- |
| Source SHA | `d8467e89ecc3a225f33eb647011f50e514a025b3` |
| DLL | `spellwheel.dll`, **8,034,304 bytes** |
| DLL SHA-256 | `7e41a2b3ab5e6306815cfe175bc75a9c53629dee61637f8803b512c4978354df` |
| Target/profile/features | `x86_64-pc-windows-msvc`, release, default features `[]`, diagnostics disabled |
| Downloadable local bundle | `spellwheel-panel-candidate-windows-x64.zip` |
| Provenance/checksums | `build-provenance.json`, `SHA256SUMS.txt` |
| Review/support material | `source.patch`, this report, usage/specification, test/check/build logs |

The ZIP contains **no TOML to install**. Its DLL is byte-identical to the standalone candidate. Build provenance records source/base SHAs, exact command, tools, lockfile hash, DLL hash and limits on verification. The working tree was clean at build time. This is a Linux cross-build of a Windows DLL, not a GitHub-hosted native build or a production release.

## Game acceptance, limitations and rollback

**This candidate has not been tested in Elden Ring by the user.** The accepted baseline release's prior successful user test does not validate this new UI/input-ownership implementation. HDR brightness/saturation, layout/readability in bright/dark scenes, native save/replace behavior, native Alt+F4 handling, actual controller suppression/release, QuestPath/FG compatibility and gameplay cycling require acceptance on the user's machine.

Controller navigation of the panel is **unavailable** because the existing infrastructure has no controller-to-ImGui navigation backend. Keyboard/mouse navigation is implemented; gamepad actions are suppressed while open. Advanced now uses native multiline text rendering; the former custom syntax-color overlay is not retained. There is no Controls/Information tab, preview studio, external shortcut conflict detection, new opening chord, or external-mod patch, as scoped.

External edits are content-checked before replacement, but ordinary filesystem rename is not compare-and-swap against another process writing in the tiny interval after the final check. Complete external editor writes before explicitly overwriting. Windows replacement/permission behavior is compiled and host failure paths are tested; it still needs native acceptance.

Incremental acceptance:

1. Close the game. Back up active DLL and TOML outside every mod-loading directory. Keep one active Spell Wheel DLL; replace **only** that DLL with the candidate. Preserve TOML. Establish FG state before launch.
2. Open with Left Shift + F10. Inspect tabs, independent sections/focus, footer/scrolling, Advanced/Graphical round trips, and HDR readability. Try reversible Appearance/HDR edits, Discard, Save, invalid TOML and both reset scopes/Cancel/Discard.
3. Exercise dirty-close Continue/Discard/Save and explicit disk reload/conflict handling using a backup. Verify comments, unknown settings, controller delay and mod paths survive. Saved Advanced technical/path changes need restart.
4. Save/release/reopen with a new binding; check None, left-side modifiers, both release orders, held keys, focus regain and failed Save retaining access. Test Alt+F4 only after basic access/recovery is established.
5. Close settings; verify original controller spell/item cycling and keyboard/mouse gameplay resume without menu actions selecting wheel entries. Then check with QuestPath and desired FG already configured before launch. **Do not switch FG live or repeat external crash testing.**

Rollback: close the game; replace only the candidate DLL with the accepted normal release at https://github.com/BandarasX/SpellWheel/releases/tag/settings-shortcut-fix-fccaa3b . Accepted DLL SHA-256: **`fe16ab221b2d2e27a47aa07bd3d21bc987f82aa1f0b5622ad52e4c9aac5a634f`**. Keep TOML; the old build ignores the new keys and uses Left Shift + F10. Restore the backup TOML only if desired. Do not delete settings or leave backup DLLs in mod-loading directories.

The FG investigation disposition is unchanged: the user previously reported the live-switch failure with Spell Wheel removed, so Spell Wheel was not necessary for that reported failure. Cause remains unresolved. No new controlled crash, FG crash patch, external report/message, or ERSS/QuestPath/ME3 change was performed for this panel task. Production publication and final in-game acceptance remain separate steps.
