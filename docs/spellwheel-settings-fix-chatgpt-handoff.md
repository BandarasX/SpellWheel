# ChatGPT handoff: Spell Wheel disappearing settings editor

Prepared 15 November 2026. This report is self-contained. It separates source facts, game evidence, implementation, and publication status; access to the prior chat or private tracker is unnecessary.

## Outcome and publication status

The settings editor's disappearing behavior was diagnosed with a reproducing game trace, fixed, and successfully retested by the user. One shortcut release was previously read by both an outer and a nested rendering callback, opening then closing the same editor 1.036 ms apart. The candidate applies each shortcut activation once per editor. The successful game trace preserves the nested callbacks, yet the editor stays open until the next deliberate shortcut.

The user confirmed: **“The editor stayed open for the duration and closed normally. Controller spell/item cycling worked normally.”** The successful trace shows 29.880140 seconds between open and explicit close.

Repository: https://github.com/BandarasX/SpellWheel. Original upstream: https://github.com/notahackusator/SpellWheel. The publication commit, Windows workflow run, release links and downloadable DLL hash will be recorded here after the publication workflow completes. The user's latest request explicitly authorizes updating the GitHub repository and making the fixed DLL downloadable; earlier investigation-only restrictions against publication no longer apply to this step.

The repository update contains the tested settings-shortcut implementation, opt-in diagnostics, regression tests, investigation records and a Windows build/release workflow. No graphical settings panel, external-mod patch, controller redesign, or fix for live-FG-switch crashes is included.

## Original problem and boundaries

The existing TOML editor, opened by Left Shift + F10, flashed and immediately disappeared in the user's setup with ERSS Frame Generation enabled and QuestPath enabled in the ME3 profile. Spell Wheel was loaded through Elden Mod Loader. The user reported the reproduction boundary as launching through that ME3 profile with QuestPath enabled. This did not independently establish which external component was responsible.

A separate reported issue is that changing FG through ERSS while QuestPath is enabled crashes the game; after restart the requested FG state is applied. That crash was neither used as a routine reproduction step nor investigated/fixed here. Never intentionally reproduce it to test the settings editor. Establish FG state before testing and restart between conditions.

An earlier duplicate-wheel incident was separate: EML had loaded both an original/backup DLL and the HDR DLL. The user moved the backup outside EML's mod directories and reported no duplicates. This investigation did not assume that incident had recurred. Two swap chains in the captured Present path are not evidence of two Spell Wheel DLLs.

## Baseline and preservation requirements

- Actual initial checkout HEAD: `76556aa96c52842a46ed672814dbb771546d784c`, on local branch `work`, clean. It exactly matched the known in-game-tested HDR build; no intervening source changes were overwritten.
- Historical tested HDR workflow run: https://github.com/BandarasX/SpellWheel/actions/runs/42289752446. Its earlier recorded successful SHA was the full baseline above. Do not substitute `4dd809d` as the tested baseline.
- Working branch for investigation: `investigate/settings-editor-diagnostics`.
- Preserve user TOML/settings, working HDR path and original controller spell/item cycling. No config defaults, shaders, HDR blending, controller cycling or external launcher settings were changed by the shortcut fix.
- The previously user-tested HDR setting was 400 nits. This is earlier HDR evidence, not a fresh comprehensive HDR acceptance test of every new binary.
- User's exact affected-run ERSS/QuestPath/ME3/game versions, FG provider/mode and complete module inventory remain unrecorded. The supplied diagnostic modules identify themselves; installed DLL byte hashes were not independently supplied.

## Source diagnosis

At the original baseline, `ListenerStatus` in `src/keyboard/listener.rs` was an `Arc<AtomicBool>`. The game task set it true when both chord keys had been down and at least one was now up. The following poll ordinarily set it false. `is_active()` only loaded this value; it did not consume an event.

`DisplaySettings::update` was called inside the ImGui Settings-window render closure and toggled `shown` every time it read true. Each RenderHook owned its editor; the source did not recreate that editor every frame. The periodic TOML reload updated the settings cache, not editor visibility. Save did not close the editor. A source search found no independent timer or automatic-close assignment responsible for the flash.

DX12 Present renders the overlay and then calls the chained Present. In the user's affected trace, that chain entered a second hooked Present for another swap-chain address, on the same thread, before the first Present returned. Both callbacks reached the same editor before another input poll cleared the release flag.

Source also showed a distinct focus edge case: original key sampling was gated by whether the foreground window belonged to the game process. Losing focus while both keys had been sampled down could satisfy the release predicate. That was not the trigger in the reproducing trace; focus stayed on the game through the recorded double-toggle.

Useful baseline references: `src/keyboard/listener.rs:30–36,44–78`; `src/settings/display_settings.rs:72–105`; `src/rendering/render_hook.rs:196–214`; `src/lib.rs:216–265`; `src/keyboard/mod.rs:263–270`; `hudhook_fork/hudhook/src/hooks/dx12.rs:208–257`. These line numbers refer to the original baseline, not the final edited files.

## Failing runtime capture

User report: **“The editor flashed and disappeared. QuestPath and FG were enabled.”**

Input files: `settings-diagnostics-32540-7ffae7930000.log` and the matching `spellwheel.log`. The diagnostic module identified base HEAD plus patch `013686fd6e1d59ec09c7d66e31bab32a267cc0628bbba82793c0d6cfd9df935f`, PID 32540, module base `7ffae7930000`, and the active `Game/mods/spellwheel.dll` path.

| Monotonic time, microseconds | Evidence |
| ---: | --- |
| 17,661,403 | One release publication: F10 up, Shift still held, focus true. |
| 17,662,759 | Present 1571 opens listener/editor `2104d5d8a90`; game tick 786; stable even poll revision 1572. |
| 17,663,795 | Nested Present 1572 closes the same editor; same game tick and stable revision 1572. |
| 17,674,475 | Next input poll clears the release flag. |

The editor opens and closes **1,036 microseconds apart**. The first draw produces 5,748 vertices; the second sees `shown=false` and no editor geometry. Both Present calls return success. Thus the disappearance is an actual state change back to closed, not just missing presentation while `shown` stays true.

The capture contains one release publication and two opposite toggles, with zero dropped events and no exhausted cap. Its later focus loss is unrelated to the toggle. Startup initialization-incomplete render errors occurred well before the shortcut and do not explain the recorded close.

Diagnostic log SHA-256: `558148c42e9ff9ad25765a70a6b517c7b5e514de66f27de550d36b63fdf42ab1`. Matching ordinary log: `8173ad379b777bac756d04b0283571d5beaf35d5e36be7cce6d2ffc2d5016be3`.

## Implementation

The behavior change is settings-specific:

- `src/keyboard/settings_shortcut.rs` holds a game-input chord state, monotonic wrapping activation generation, and per-editor consumption cursor.
- The existing input task invokes its poll through `src/keyboard/listener.rs`. It takes one foreground-process focus snapshot and samples Left Shift and F10. A valid transition from both down to either released publishes one generation.
- `DisplaySettings` uses `SettingsShortcut::update_visibility` instead of repeatedly reading the generic release boolean. The cursor applies each generation once. Extra or nested rendering callbacks seeing the same generation do nothing.
- Pending activations survive later input polls when rendering is delayed. Multiple deliberate chords between draws retain their combined toggle effect through generation-difference parity.
- Startup/focus regain requires both keys released before arming. Focus loss discards an incomplete chord without publishing it. This avoids the independently source-proven phantom-release case.
- A second deliberate chord still closes the editor; either key-release order works. Holding keys does not repeat.
- Generic Pressed/Held/Released listener publication and read semantics are preserved for other consumers. No swap chain is arbitrarily suppressed or selected to debounce the shortcut.

The main relevant edited-code entry points are `ShortcutState::poll`, `ActivationCursor::apply`, `SettingsShortcut::update_visibility`, the settings poll in `tick_listeners`, and `DisplaySettings::update`. The render-hook changes only update diagnostic instance references; the HDR renderer/backends retain their prior behavior except optional observations.

## Successful candidate test

The candidate's delivered DLL SHA-256 was `9819d8a6049966e86696e9c86d855609ed8aa6927465050a8f997a7fb5c3b4b5`. Its cumulative source patch SHA-256 was `565960d0f69b609549a132470a72155112d6e025931970965bc95983f3b1584c`; its incremental fix patch against the original diagnostic snapshot was `0d2a7a9a833425b61b698364e01153a9e03afd8e4765ec0202dfe54bf004729b`.

The subsequent log `settings-diagnostics-5440-7ffae8840000.log` reports that candidate source identifier, PID 5440, module base `7ffae8840000`, editor identity 1:

| Monotonic time, microseconds | Evidence |
| ---: | --- |
| 20,319,759 | Generation 1 published. |
| 20,320,986 | One activation applied: closed → open. |
| 20,322,005 | Nested Present draws `shown=true` without applying the same activation again. |
| 50,199,764 | Generation 2 published by the next deliberate chord. |
| 50,201,126 | One activation applied: open → closed. Nested Present leaves it closed. |

Open interval: **29.880140 seconds**. All 44 sampled editor draws during the open interval have `shown=true`. There are 88 sampled Present entries and matching successful returns, no sampled overlay errors, no recorded size skips, and exactly two generation publications/two processed activations. Final counters: 630 attempted events, zero drops, no cap exhaustion. The trace samples render observations for limited windows; the user's visual observation establishes continued display across the full interval.

User-observed controller cycling was normal. The user did not separately report a new HDR appearance check, both release-order runtime checks, or focus-regain/held-key runtime checks; those input-state cases have automated coverage. Do not expand this acceptance into blanket compatibility claims.

Successful diagnostic log SHA-256: `d2d966b8a9cac8249afd94e3540bbe064dd89671b433da9bdbf9ab7931a76984`. Matching ordinary log: `8e8ec82a36ecbd8d1ad6730f6241d7f364aa5ad1524fb0e8c95326fd8426e0bd`.

## Automated checks and build environment

- Thirteen passing host tests import the actual settings-shortcut and listener source. They cover the observed nested-render schedule and normal close, repeated render reads, held keys, both release orders, focus/startup re-arming, pending events across missing draws, multiple activations, independent editor cursors, generation wrap, actual game-poll-to-editor integration, and preserved generic listener behavior.
- Three passing tests exercise the actual diagnostic transport: concurrent event cap, queue-full drops without blocking, and nested Present correlation restoration.
- Locked MSVC x64 default and diagnostic release cross-builds passed. The all-features release check passed. PE inspection identified an AMD64 executable DLL. Whitespace and source-patch application checks passed.
- Local tools: pinned `nightly-2026-11-13`, rustc `1.102.0-nightly (8eb778d04 2026-11-12)`, cargo-xwin 0.20.2, clang-cl/LLD 19.1.7, existing cached Windows SDK. A local `CL=/FIstring.h` workaround accommodates the existing libudis86-sys C header flags under clang-cl; dependency source and root Cargo.lock were not changed.
- Root Cargo.lock SHA-256: `bed1ccc00bec7e9d5a74cd9166bcd4d28f61b2a5fd46d9943d932298e1ca12eb`.
- The Linux Cloud environment cannot run Elden Ring/ERSS/QuestPath. Runtime evidence comes from the user's actual game tests, not from compilation or a synthetic replacement for gameplay.

Reproduce the host checks with `cargo +nightly-2026-11-13 test --locked --manifest-path tests/listener-harness/Cargo.toml`, and compile/run the standalone transport tests using `rustc --test --edition 2021 hudhook_fork/hudhook/src/settings_diagnostics.rs`. For local MSVC cross-builds, source `/workspace/.spellwheel-tools/activate.sh`, set `CL=/FIstring.h`, and use `cargo +nightly-2026-11-13 xwin build --release --locked --offline --target x86_64-pc-windows-msvc` with optional `--features settings-diagnostics`.

## Diagnostics and end-user installation

Diagnostics require both the compile-time `settings-diagnostics` feature and an empty `spellwheel-settings-diagnostics.enable` marker beside the active DLL, checked at startup. Normal/default release builds compile diagnostics out; no marker is needed. Removing the marker and restarting disables file logging in the user-tested diagnostic candidate while retaining the shortcut fix.

The diagnostic trace records only shortcut bits/focus, module/instance identity, generations/toggles and bounded render/presentation observations. It does not record typed editor text or arbitrary key messages. It caps attempted events at 4,096, queues at most 256, samples the first 24 Presents and then twice per second for ten seconds after activation, and flushes nonempty batches at most once per second from the game task. No per-frame render-thread file writes or new worker thread were introduced. A stalled tick can delay the final batch; dropped/capped/incomplete traces cannot establish an event's absence.

For installation, close the game, replace only the active `spellwheel.dll`, keep backups outside every mod-loading directory, and preserve the existing TOML. The default release includes the fix without diagnostic logging. Do not replace the user's settings file with defaults. Do not change FG live as part of installing or testing this fix.

The newly published native Windows DLL is rebuilt from the committed fix with diagnostics disabled. It will have a different hash from the previously user-tested diagnostic cross-build. The successful candidate test validates the fix's source/behavior in that setup; it does not constitute a separate game test of the newly packaged native DLL.

## Repository publication process

The GitHub connector supports repository Git tree/commit/ref operations and reads, but not release uploads. Local GitHub CLI API access was unavailable. Publication therefore uses the connected GitHub tools to write a commit and fast-forward `master` only if its expected prior SHA still matches, without force-overwriting intervening work. A push-triggered Windows GitHub Actions workflow performs the native build and release upload.

`.github/workflows/build-hdr.yml` runs both test suites before every build, pins nightly-2026-11-13, and retains manual diagnostic builds. Relevant pushes to master publish a commit-specific `settings-shortcut-fix-<short-SHA>` release. Documentation-only pushes do not rebuild. A manual normal build can explicitly request publication; diagnostic builds are not published as normal releases.

`.github/scripts/package-release.ps1` packages `spellwheel.dll`, build provenance, install instructions and the DLL checksum in a ZIP, plus a SHA256SUMS file for both direct DLL and ZIP downloads. Release assets identify the exact source commit and workflow run. This provides a durable GitHub download instead of relying on expiring Actions artifacts.

Historical reports in this directory describe the state when each investigation/test was performed. Statements in them such as “uncommitted,” “not released,” or “runtime not run” are historical, not the current publication status. This handoff's completed publication record is authoritative for the final repository/download state.

## Remaining work and approved later panel context

The target disappearing-editor issue passed the user's affected-setup test, including normal close and original controller cycling. The separate live-FG-switch crash is still unresolved and uninvestigated. No independent component attribution was established for the additional Present call. No new graphical settings panel was implemented.

The planning assistant may record this outcome in the private tracker at https://spell-wheel-hdr-lab.lovablebonus.chatgpt.site; it was not accessed or updated here. Planning/design and approval belong in ChatGPT, implementation/checks in Codex Cloud, and real-game acceptance with the user.

For future work, the earlier approved panel direction remains:

- Appearance, HDR and Interface tabs; related sections can independently collapse, without nesting; common actions stay visible.
- Advanced opens the existing TOML editor beside Discard, not as a tab; its return action is Graphical mode. Both modes edit one shared draft, preserve comments/unmanaged settings, validate text when returning, and never silently save/drop invalid edits.
- Separate saved, draft and effective-preview values; reversible preview; explicit Save/Discard; no success message after a failed write; coordinate periodic disk reload with unsaved edits.
- Interface initially supports F1–F12 and at most one of None/Left Shift/Left Ctrl/Left Alt. Default remains Left Shift + F10. A new binding takes effect only after successful Save and key release. No conflict detection or F13–F24 scope is assumed.
- Reset stages defaults in the draft, defaults to the current tab with an all-panel option and confirmation, remains reversible, preserves unmanaged paths/settings and external mod settings, and requires explicit invalid-TOML recovery.
- Verified project defaults for the later panel are HDR enabled, 400 nits, saturation 1.0, and Left Shift + F10. The previously supplied user TOML is not a project-defaults table; for example its 0.25 controller-open delay was user customization.
- Preserve original controller cycling outside the menu; later menu interaction should not leak gameplay actions. Do not imply opening the overlay pauses the game.
- Later order: graphical panel, controller input handling improvements, richer information. A sample wheel preview remains a later addition.

Any follow-up should start from the published repository state and actual HEAD, preserve this shortcut regression coverage and HDR behavior, and avoid reintroducing render-frequency-dependent activation.
