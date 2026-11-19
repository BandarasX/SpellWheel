# Spell Wheel settings panel

Download the Windows x64 build from [the GitHub release](https://github.com/BandarasX/SpellWheel/releases/tag/settings-panel-56c8ed2). See the [implementation handoff](spellwheel-panel-chatgpt-handoff.md) for acceptance results and build provenance.

## Using the panel

Open with **Left Shift + F10** by default. Appearance, HDR and Interface contain independent collapsing sections; the main section starts expanded. Use mouse controls, or Tab/Shift+Tab, arrows, Space and Enter for keyboard navigation. Left/Right collapses/expands a focused section header and keeps focus there. Native ImGui remembers sections and the selected tab while editing. The content scrolls and the two-row footer remains accessible. The panel opens near the left edge at 620 × 560 on a 3440 × 1440 display. Drag the title bar to move it or the lower-right corner to resize it.

The footer has **Reset to Defaults…**, an explicit **Reload disk…** action, then **Save | Discard | Advanced mode**. Advanced mode uses the existing ImGui multiline TOML editing widget and returns through **Graphical mode**, immediately beside Discard. It is one document/draft, not a second configuration. Native multiline selection, scrolling and keyboard editing replace the previous custom syntax-color overlay; errors are shown in the persistent, scrollable feedback area.

Save validates and persists, leaves the panel open and reports the actual result. Discard restores the last successfully saved document and visual values, including Advanced edits and staged resets. Closing a dirty draft asks **Save and close / Discard and close / Continue editing**. Invalid text cannot be saved, switched to Graphical or reset; fix it or explicitly Discard. Focus loss never saves/discards/toggles. Shortcut activations are consumed while a modal is open so one chord cannot also choose a modal action.

The overlay **does not pause the game**. Keyboard/mouse navigation is implemented. There is no existing controller-to-ImGui navigation backend; controller navigation of the panel is therefore unavailable in this version. While settings capture interaction, window input, existing DirectInput/raw-mouse paths and gamepad state are suppressed for the game; Spell Wheel selection/cycling hooks also respect ownership. Outside settings, the original cycling logic is retained.

## Preview, reload and persistence

Only the managed Appearance/HDR fields preview in memory. The preview opens and closes with the panel. It keeps the wheel and input style already visible on opening. Otherwise it shows the quick-item wheel with the controller-style ring, falling back to spells when there are no quick items. Selection stays frozen. Layout changes rebuild existing wheel geometry/text wrapping as needed. HDR changes use existing shader constants, including brightness and saturation, without recreating DX12 resources per edit. HDR appearance requires a real supported HDR output; the panel does not simulate HDR or change screen mode, ERSS, FG or other mods.

Invalid text retains the last valid preview. Periodic disk refresh is suspended while the panel is open. The draft separately watches disk changes; no external edit replaces a draft automatically, even when clean. Save rechecks the disk baseline. If changed, choose explicit **Overwrite disk**, **Reload disk (discard draft)**, or **Keep editing**. Reload disk is a read, never a Save. An unreadable file retains runtime state and blocks editing/Save until Reload disk succeeds.

Save uses a uniquely named `create_new` temporary file beside the TOML, flushes/syncs and closes it, rechecks the disk baseline and replaces via `std::fs::rename` (Windows `MoveFileExW(REPLACE_EXISTING)`). Failed read/write/replace retains the original file, saved state, draft and active saved binding, and cleans the temporary file. This does not provide compare-and-swap against another process writing in the tiny interval between the final content check and replacement; external editors should finish writes before choosing Overwrite.

A successful **Save** applies wheel enable switches and bindings, cursor centering, instant switching, mouse/controller selection thresholds, controller open delay, and debugging in the current session. To turn on debugging in Advanced, set `debugging = true` at the top level before any `[section]` headers, then save. The default is `false`.

Only changes to `mods`, `modded_spells`, `force_rgba`, `timing_offset`, and `await_xinput_hook` require a game restart. **Reload icons** is available in Advanced for valid saved settings and the active icon paths and texture format. Save and restart to apply path or encoding changes.

## New TOML keys and recovery

```toml
settings_function_key = "F10" # F1 through F12, exact spelling
settings_modifier = "left_shift" # none | left_shift | left_ctrl | left_alt
keyboard_mouse_inner_ring = false # Appearance → Wheel layout
```

A draft binding becomes active only after successful Save (or explicit acceptance of a persisted disk document), then release of the old/new relevant keys. Both release orders work; held keys do not repeat. Repressing the function key while retaining the selected modifier is another intentional chord. None rejects extra Shift/Ctrl/Alt, and modifiers require the selected left side without extra modifiers. Startup/focus regain requires release before rearming. Monotonic activation generations and per-editor cursors remain; rebind epochs invalidate queued old activations. Nested rendering cannot consume one generation twice.

Left Alt + F4 has a narrowly scoped native system-key filter to prevent the configured chord from generating Windows close commands. This does not intercept ordinary window-close commands or perform shortcut-conflict detection.

Invalid startup hotkey values, including wrong types, recover **only the binding in memory to Left Shift + F10**, log the reason, and preserve unrelated values and the original file. The panel opens the unchanged invalid document in Advanced for repair. Recovery while the game is closed: back up TOML, correct the two fields above (or remove them for the default), then relaunch. Other invalid startup settings log an error and use compatibility defaults in memory; they are never silently written to disk. Invalid later disk refreshes keep the last valid runtime.

## Project defaults and reset scope

Existing explicit values are preserved. Existing documents with omitted keys still use compatibility fallbacks (HDR false / 200 nits). A genuinely missing new config and an explicit panel reset use `project_defaults()`: HDR enabled / 400 nits / saturation 1.0. Merely opening settings never writes a file or forces an existing document to 400 nits. `build/spellwheel.toml` documents the new-config project defaults.

| Managed key | Project reset value |
| --- | --- |
| `style` | `"pretty"` |
| `keyboard_mouse_inner_ring` | `false` |
| `icon_scale_multiplier` | `0.15` |
| `radius_multiplier` | `0.3` |
| `highlight_expansion` / `highlight_time` | `0.15` / `0.15` seconds |
| `item_names` | `"show"` |
| `font_scale_multiplier` / `text_shadows` | `1.0` / `true` |
| `icon_tint` / `icon_background_tint` / `name_tint` | each `[1.0, 1.0, 1.0, 1.0]` |
| `hdr_enabled` / `hdr_ui_brightness` / `hdr_ui_saturation` | `true` / `400.0` nits / `1.0` |
| `settings_function_key` / `settings_modifier` | `"F10"` / `"left_shift"` |

Reset asks for Current tab by default in Graphical; Advanced explicitly offers All panel settings. All-panel confirmation names the HDR/hotkey effects. Confirm stages only managed keys; Cancel changes nothing, Save persists and Discard reverses. Unknown keys, comments, mod paths, controller delay (including a customized 0.25), and technical fields are excluded. No nested sections, Controls/Information tab, external settings or automatic restart are included.

## Numeric domains and units

Percentage controls offer sliders with 0.5-point steps and exact-entry fields. The idle fields show two decimal places and preserve stored precision. Exact entry allows validated values outside the slider range. 100% icon size is multiplier **0.15**, 100% radius is **0.3**, font/saturation 100% is **1.0**, enlargement 15% is **0.15**. Duration is seconds and brightness is nits. Pretty-only enlargement, transition and icon-background tint are disabled/annotated in Simple without resetting them. Style/name placement retain the renderer's case-insensitive compatibility.

All known numeric values must be finite. Supported panel geometry budgets are icon multiplier **0.001–4**, radius **0.001–8**, font **0.01–16**, enlargement **0–8** (0–800%), and transition **0–60 seconds** (zero is immediate). These are conservative geometry/text-scale budgets, not ordinary slider bounds. The renderer multiplies sizes by the smaller viewport dimension and highlight scale by `1 + expansion`; the caps keep screen-space sizes and text scales modest enough for finite calculations through 16384-pixel desktop dimensions. Independently, radius **must exceed icon multiplier + 0.01**: selector radius subtracts icon size and twice `screen_min / 200`; equality/negative clearance degenerates the selector. Original rendering equations and HDR shaders are preserved.

Brightness **50–2000 nits** and saturation **0–2** match the existing DX12 clamping domains; RGBA tints are **0–1**. Unmanaged delay/selection/timing numbers must be finite/nonnegative; squared selection distances must remain finite at the same 16384-pixel envelope, and timing must fit a Rust Duration. They have no new UI controls/reset behavior. Out-of-domain existing text stays intact in Advanced with a named-key error; no silent clamp or config rewrite occurs. Values beyond typical UI suggestions but inside these domains remain editable and unchanged when merely viewed.

## Automated checks and real-game acceptance

Host tests import the production values, document model, shortcut/listener and actual ImGui panel source. They run without a GPU, game, Windows input devices or HDR output. The original 13 listener/shortcut regressions remain, expanded to all 48 supported bindings, save/rebind/failure, document and reset semantics, input policy, and actual ImGui modal/footer/focus interactions. The independent three diagnostic-transport tests remain unchanged. Native Windows CI runs the same harness before building each release.

```bash
source /workspace/.spellwheel-tools/activate.sh
cargo +nightly-2026-11-13 test --locked --offline --manifest-path tests/listener-harness/Cargo.toml
rustc +nightly-2026-11-13 --test --edition 2021 hudhook_fork/hudhook/src/settings_diagnostics.rs -o /tmp/settings-diagnostics-tests
/tmp/settings-diagnostics-tests
CL=/FIstring.h cargo +nightly-2026-11-13 xwin check --locked --offline --all-features --target x86_64-pc-windows-msvc
CL=/FIstring.h cargo +nightly-2026-11-13 xwin build --release --locked --offline --target x86_64-pc-windows-msvc
```

The user passed all nine required checks on final candidate `0bb5bab`. The optional blocked-Save check was skipped in-game and covered by automation. Use the following checks if you want to verify the release on your setup.

1. Close the game, back up TOML and the active DLL **outside every mod-loading directory**. Preserve TOML; install only the release DLL. Ensure exactly one active Spell Wheel DLL. Set desired FG state before launch.
2. Open settings; inspect tabs, multiple independent sections, keyboard focus, small-window scrolling/footer and Advanced/Graphical round trips. Check bright/dark HDR scenes on the actual display.
3. Change appearance/HDR and verify reversible preview/Discard, then Save without closing. Try invalid TOML, both reset scopes, reset Cancel and Discard, and dirty-close Continue/Discard/Save. Verify controller delay/mod paths/comments remain unchanged. For deliberate save-error/conflict checks, use a backed-up test config and restore access before continuing.
4. Save a new binding, release old/new keys, close/reopen. Try both release orders, held keys, focus loss/regain and a failed Save retaining the old binding. Verify None rejects extra modifiers; test Alt+F4 only after basic access/recovery succeeds.
5. Close settings and verify original controller spell/item cycling and keyboard/mouse gameplay resume, with no menu inputs selecting wheel entries. Finally verify the same flows with QuestPath and FG already configured **before launch**. Do not switch FG live or reproduce external crashes.

Rollback: close the game and replace only the release DLL with the accepted `settings-shortcut-fix-fccaa3b` DLL (SHA-256 `fe16ab221b2d2e27a47aa07bd3d21bc987f82aa1f0b5622ad52e4c9aac5a634f`). Keep the current TOML; the old build ignores new unknown hotkey keys and uses Left Shift + F10. Restore the backed-up TOML only if desired, without deleting settings or putting backup DLLs into a loading directory.
