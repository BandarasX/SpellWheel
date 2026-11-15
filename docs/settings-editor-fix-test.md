# Settings shortcut fix — candidate test build

> Historical investigation/test record. See [the final ChatGPT handoff](spellwheel-settings-fix-chatgpt-handoff.md) for the completed publication status.

This candidate addresses the double-toggle captured in the user's QuestPath-enabled, FG-on run. It is a test build, with opt-in diagnostics retained. Windows/game acceptance is still outstanding; no merge or release was performed.

## What changed

`src/keyboard/settings_shortcut.rs` provides a settings-only chord state and activation counter. The game task samples Left Shift/F10 and foreground-process focus once per poll, publishes one new generation for a valid chord release, and retains that generation through later polls. Each editor remembers which generation it processed. Repeated/nested render callbacks observing the same generation cannot toggle it again. Multiple deliberate chords between draws retain their combined toggle effect; input is not cleared just because no draw occurred.

The chord activates when either key releases after both were sampled down. Held keys do not repeat. At startup and after focus loss, the settings chord waits until both keys are released before arming; focus loss discards an unfinished chord without creating a release event. This focus handling is settings-specific.

`src/keyboard/listener.rs` invokes the new settings poll from the existing game task. The other listeners' boolean publication/read behavior is preserved. `DisplaySettings` uses the new per-editor cursor instead of its old shared release flag. Rendering diagnostics use that cursor's instance identity. No HDR shader/backend, controller/XInput/cycling logic, TOML settings/defaults, external mod or graphical panel change is included in the fix.

## Checks and provenance

The candidate is based on HEAD `76556aa96c52842a46ed672814dbb771546d784c` plus the existing diagnostic diff and the new settings-shortcut fix, all left reviewable and uncommitted. The original diagnostic bundle remains unchanged. This candidate has its own archive, source patch hash, embedded build label and DLL hash; consult `build-provenance.txt` in this bundle rather than the original build's record.

The host harness imports the actual settings shortcut and listener source. Its tests cover the captured outer/nested-render schedule, subsequent explicit close, many repeated reads, held keys, both release orders, startup/focus re-arming, pending events across missing draws, multiple activations, independent editor cursors, generation wrap, game-poll-to-editor integration, and unchanged Pressed/Held/Released listener semantics. The diagnostic transport's three existing tests are also run. Build/check outcomes, exact commands and tool versions are recorded in the bundle's logs and provenance. Neither automated tests nor compilation validate the game's final HDR output or prove compatibility with external mods.

## What to do

1. Exit Elden Ring completely. Extract the **new candidate** archive and replace the currently active `spellwheel.dll` with its DLL. Keep all backup DLLs outside mod-loading folders. Keep the existing `spellwheel.toml` unchanged.
2. Keep the existing empty `spellwheel-settings-diagnostics.enable` file beside the active DLL. If it was removed, recreate it with that exact name and no `.txt` suffix.
3. Launch the same ME3 profile with QuestPath enabled and FG already enabled for this run. Do not switch FG while the game is running.
4. In the same playable scene, release all keys. Hold Left Shift, press F10, hold both for about half a second, release F10, then release Shift. **Expected: the editor opens and stays open for at least 12 seconds.** Do not edit or save the TOML during this test.
5. Press/release the same chord a second time. **Expected: it closes once and stays closed.** Wait 12 seconds for the trace.
6. Try one further open/close pair with Shift released before F10. It should behave the same. Holding both keys for a second should not repeatedly toggle the editor.
7. Briefly verify ordinary controller spell/item cycling and the existing wheel/HDR appearance with the settings editor closed. Do not change brightness or other settings for this comparison. If anything regresses, report what you observed.
8. Leave the game in the playable scene for two seconds to flush logs, then exit normally. Send the new `spellwheel/settings-diagnostics-<pid>-<module-base>.log` and `spellwheel/spellwheel.log` from beside the active DLL, plus whether open, close and controller behavior worked.

No further QuestPath/FG configuration comparisons are required for this first acceptance test. Any configuration changes should be made only by the user and tested after restart, without deliberately reproducing the reported live-FG-switch crash.

The new trace should identify the candidate's source patch in `module_start`. Settings release publications now include `source=settings_shortcut` and `generation`. An editor activation record uses `reason=shortcut_generation`, `activations` and `generation`; one deliberate chord should produce one processed activation, with no second toggle from the nested Present. If several deliberate chords were queued before a draw, one batch can report their combined count. A valid short capture should show `dropped=0 capped=false`.

If the candidate fails, exit the game and restore the saved prior DLL from outside the loader directories. Preserve the logs before another launch. Removing the `.enable` marker and restarting disables file diagnostics; restoring the original known-good DLL returns to its original shortcut behavior. The candidate does not automatically modify or save settings.
