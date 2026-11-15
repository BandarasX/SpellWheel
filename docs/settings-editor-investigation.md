# Spell Wheel settings editor investigation — 15 November 2026

> Historical investigation/test record. See [the final ChatGPT handoff](spellwheel-settings-fix-chatgpt-handoff.md) for the completed publication status.

The source contains a concrete way for one shortcut release to toggle the editor multiple times: the game task publishes a release flag, and the render loop repeatedly loads that flag without consuming it. This mechanism is verified by source and host tests. Whether it explains the user's disappearing editor with QuestPath enabled and FG on remains unverified. A diagnostic DLL and distinguishing test procedure are provided; no behavioral fix is applied.

## Checkout and scope

- Repository: BandarasX/SpellWheel. Initial branch: `work`, clean working tree.
- Actual HEAD and tested HDR comparison base: `76556aa96c52842a46ed672814dbb771546d784c`. They are identical; there were no intervening source changes to compare.
- Review branch: `investigate/settings-editor-diagnostics`. Changes are an uncommitted reviewable diff over that HEAD. The bundle includes `source.patch` and its SHA-256 in `build-provenance.txt`.
- No AGENTS.md applies to this checkout. The existing local tool setup was reused.
- No game/launcher/mod configuration was changed, no cloud task or CI run dispatched, and no commit, merge or release performed. No graphical panel or controller behavior change was implemented.

## Source evidence

References in this table are to the unmodified HEAD above, so they remain stable independently of diagnostic line additions.

| Fact | Baseline source reference |
| --- | --- |
| `DisplaySettings::new` registers one fresh `Released` listener for Left Shift + F10 and initializes `shown=false`. | `src/settings/display_settings.rs:72–92` |
| `DisplaySettings::update` toggles whenever `is_active()` returns true. This is the only assignment changing the existing editor's visibility; there is no timer, focus-close callback, Save-close action, or periodic-reload close. `IS_OPEN` mirrors `shown`. | `src/settings/display_settings.rs:95–105`; full-tree visibility search |
| `ListenerStatus` wraps an `Arc<AtomicBool>`. `is_active()` performs a load, without clearing the flag or tracking a consumer. | `src/keyboard/listener.rs:30–36` |
| A polling pass sets a Released listener true when all chord keys were down in the previous pass and at least one is now up. The following ordinary pass sets it false. Auto-repeat messages do not drive this polling algorithm. | `src/keyboard/listener.rs:44–78` |
| The game task calls `tick_listeners()` after several singleton/resource checks. Early returns skip polling and can extend the life of a previously published flag. | `src/lib.rs:216–265` |
| `RenderHook` owns its `DisplaySettings`; `new` creates it. Normal initialization creates one RenderHook per DLL via the guarded `try_init_rendering`, and DX12 keeps one render loop/pipeline. The listener is not recreated each frame. Registered listeners remain in the registry; no unregister path exists. Multiple loaded DLLs would have separate statics, but there is no new runtime evidence of multiple DLLs in this case. | `src/rendering/render_hook.rs:21–43`; `src/rendering/mod.rs:18–45`; `hudhook_fork/hudhook/src/hooks/dx12.rs:178–205`; `src/keyboard/listener.rs:81–96` |
| The Settings ImGui window's build closure calls `update()` and then `draw()` once whenever that closure runs. A skipped/clipped window can skip that closure; there is no separate game-task editor update. | `src/rendering/render_hook.rs:196–214` |
| DX12 hooked Present calls prepare/render, obtains the invoking swap chain's current back buffer, then calls the chained Present. This is separate from game-task polling. It does not filter later calls by the initially captured swap-chain identity. Concurrent/reentrant pipeline access may fail `try_lock`. Callback frequency alone does not establish that a release was read twice. | `hudhook_fork/hudhook/src/hooks/dx12.rs:208–257` |
| Keyboard sampling uses GetAsyncKeyState's high bit and gates it by whether the foreground window belongs to the game process. Losing focus while both keys were sampled down can therefore satisfy the release predicate. Releasing either chord key first activates the listener; releasing the remaining key on the following pass does not itself produce another activation. | `src/keyboard/mod.rs:263–270`; `src/io/mod.rs:135–142`; `src/keyboard/listener.rs:52–74` |
| Periodic settings loads replace the settings cache. They do not recreate DisplaySettings or assign `shown`. Save writes the TOML and reloads that cache. | `src/settings/mod.rs:329–373` |
| Rendering can return early for zero display/framebuffer dimensions, fail before/after the UI callback, or catch a panic inside `guard!`. `shown=true` alone does not establish a visible presented editor. | `hudhook_fork/hudhook/src/renderer/pipeline.rs:138–164`; `src/lib.rs:128–138`; DX12 render path above |

A possible interleaving is: game poll publishes true → Present A reads true and opens → Present B reads the same true and closes → next game poll clears true. An odd number of reads leaves it open; an even number leaves it closed. A release can also be missed entirely if another poll clears it before rendering reads it. These are consequences of existing semantics, not observed ERSS/QuestPath traces.

## Runtime evidence and hypotheses

Runtime evidence supplied for this investigation consists of the handoff's user reports: issue A and the separate FG-switch crash issue B share the ME3-with-QuestPath-enabled reproduction boundary. That boundary does not identify which component or interaction is responsible. No affected-run logs, installed DLL hash, current module inventory, or fresh environment capture were available in Cloud. The prior duplicate-wheel incident was resolved according to the handoff and is not treated as the current explanation.

The leading hypothesis is multiple render reads of one release interval. Alternatives include a focus-driven release/re-arm, another editor/module instance, a skipped UI closure, or draw/backend/presentation trouble while `shown` remains true. No conclusion links the FG-switch crash to the disappearing editor. No FG transition was attempted.

## Diagnostic diff

Compile-time feature `settings-diagnostics` forwards to the vendored hudhook. Runtime opt-in additionally requires `spellwheel-settings-diagnostics.enable` beside the loaded DLL, checked once during startup. Ordinary builds compile out all diagnostic sites. Feature builds without the marker create no diagnostic file.

The separate `spellwheel/settings-diagnostics-<pid>-<module-base>.log` records:

- Monotonic microseconds, Rust thread ID, callback/Present ID, module base, actual DLL/config paths, and a build revision label.
- Stable listener identity, poll revision/game tick, only Left Shift/F10 sampled bits, foreground-process focus changes, release publication, and every editor visibility toggle with before/after and reason.
- A bounded selection of Present IDs with swap-chain address, current buffer, swap-chain description, flags, render result and chained HRESULT; UI/draw-data dimensions/counts and editor state; resize calls/results.

The poll revision is odd during polling, even after all status stores. Multiple toggles for the same listener with the same **even**, unchanged before/after revision are strong evidence that multiple UI reads used one polling interval. Odd or changing revisions indicate overlap and require the surrounding events. `chord_poll publish` is logged immediately before the existing store; its timestamp is not an atomic timestamp of the store. Key bits are the existing focus-gated samples, not an independent hardware capture; the focus snapshot is taken separately and can race a focus transition. Sort concurrent events by `us`, rather than assuming file arrival order.

Logging is capped at 4,096 attempted events with a 256-entry nonblocking queue. Each release arms ten seconds of render observations: the first 24 Presents, then at most two sampled Presents per second. Toggle events remain unsampled within the overall cap. Only nonempty queued batches are written, at most once per second from the existing game tick after dropping listener locks. Render callbacks perform no file IO. There is no new background thread, keyboard hook, or asynchronous DLL lifetime to manage. No arbitrary key messages, typed TOML, or settings values are logged.

`flush_us` reports attempted events, drops and cap state. A stalled/skipped game tick delays flushing and process exit can lose the final batch. A nonzero drop count or exhausted cap makes absence of an event inconclusive; restart for a shorter capture. Diagnostic atomics, formatting and occasional game-tick batch IO can alter timing. Failure to reproduce in this build does not disprove a timing issue. Compilation cannot establish in-game/HDR compatibility.

## Checks and build provenance

- Four passing host tests import the actual `src/keyboard/listener.rs`, replacing only key/focus polling and debug-output dependencies. They characterize persistent repeated reads, held-key stability, both modifier release orders, a missed release, and focus-loss activation. They intentionally preserve current behavior, rather than assert a speculative fix.
- Three passing host tests exercise the actual shared diagnostic module: an exact concurrent event budget, queue-full drops without blocking, and nested Present correlation restoration.
- Locked offline MSVC x64 diagnostic release build and all-features release check passed with pinned `nightly-2026-11-13` and existing cargo-xwin 0.20.2. PE inspection reports AMD64 / executable DLL. See the bundle's build logs and provenance for exact commands, DLL hash and source diff hash.
- Local cross-build needed `CL=/FIstring.h` to accommodate the existing libudis86-sys C source/header flags under clang-cl. No dependency source or Cargo.lock was modified. The initial build without that workaround failed; it is not counted as validation. The compiler still reports existing project warnings and nightly deprecation warnings for `AtomicU64::fetch_update` (used to retain compatibility with hudhook's Rust 1.85 floor).
- Rustfmt was unavailable; its optional component download was blocked by the environment's network policy. `git diff --check` is used for whitespace checking.
- Windows CI now has an optional `settings_diagnostics` workflow-dispatch boolean, default false, which runs these host tests and builds a separately named diagnostic artifact. No workflow was dispatched. The actual existing default artifact name at this checkout is `spellwheel-hdr-windows-x64`; the handoff's historical `spellwheel-hdr` label has not been independently reverified.

No Elden Ring, Windows desktop, ERSS or QuestPath reproduction ran in Cloud. The next required evidence is one short affected-run capture following [the manual test procedure](settings-editor-test.md).

## How the next observation selects work

| Observation at the reported disappearance | Interpretation / next step |
| --- | --- |
| Same listener opens and closes, same stable even poll revision, different Present IDs, no second physical activation | Confirms repeated observation of one release interval. Propose an event generation with per-editor consumption and focus re-arming for separate review; verify single-toggle/held-key/focus/close requirements before implementation. |
| New chord publications or focus changes precede the close | Investigate re-arming/focus and the physical sequence; callback multiplication alone is insufficient. |
| New editor identity or multiple module identities | Correlate loader/module inventory and lifecycle. Do not infer duplicate DLLs just from filenames or callback count. |
| State stays open, but editor draw is missing | Compare render-loop begin/end, window dimensions and normal-log panic records. A skipped Settings build closure is distinguishable from a visibility toggle. |
| State stays open and draw data is produced, but render/Present returns an error | Trace the reported stage/error, swap chain and buffer. |
| State stays open, geometry exists, API results succeed, but editor is invisible | API success does not prove display. Next inspect swap-chain/buffer identity and presentation/resource ownership against the baseline; this trace cannot diagnose final compositing alone. |
| No reproduction, dropped/capped/incomplete trace, or overlapping revisions only | Do not claim resolution. Repeat one short run; the next distinguishing observation is whether `shown` becomes false or stays true when the user sees it disappear. |

Any later fix must preserve the existing shortcut's explicit close, avoid repeated toggles while held, avoid focus-created phantom activations, preserve HDR settings/rendering and original controller cycling, and receive its own review. Those changes are outside this diff.
