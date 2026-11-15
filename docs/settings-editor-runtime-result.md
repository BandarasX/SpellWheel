# Runtime result: disappearing settings editor confirmed

> Historical investigation/test record. See [the final ChatGPT handoff](spellwheel-settings-fix-chatgpt-handoff.md) for the completed publication status.

The supplied affected-run trace confirms the immediate cause of the reported flash: the same editor toggles open and then closed from one published shortcut-release interval. The second toggle occurs 1,036 microseconds after the first, before the next listener poll. This establishes the double-toggle mechanism in the user's tested configuration; a behavioral fix has not been implemented or validated.

## Evidence identity and limits

- User observation: editor flashed and disappeared; QuestPath and FG were enabled. ME3/QuestPath/ERSS causation and exact versions are not independently established by these logs.
- Diagnostic process: PID `32540`, module base `7ffae7930000`, DLL path `D:\Steam\steamapps\common\ELDEN RING\Game\mods\spellwheel.dll`.
- Source base: `76556aa96c52842a46ed672814dbb771546d784c`.
- Embedded diagnostic source patch SHA-256: `013686fd6e1d59ec09c7d66e31bab32a267cc0628bbba82793c0d6cfd9df935f`. This matches the supplied diagnostic bundle's source identifier. The installed DLL's byte hash was not supplied independently.
- Diagnostic log SHA-256: `558148c42e9ff9ad25765a70a6b517c7b5e514de66f27de550d36b63fdf42ab1`.
- Ordinary log SHA-256: `8173ad379b777bac756d04b0283571d5beaf35d5e36be7cce6d2ffc2d5016be3`.
- The diagnostic trace contains one module-start record, one editor creation, one release publication, and two editor toggles. Its final flush reports 319 attempted events, zero drops and no exhausted cap. This identifies the responsible editor instance; it does not replace a complete process module inventory or prove that no other DLL is loaded.
- The normal-log release activation is at `2026-11-15T11:08:24.141518Z` (20:08:24.141518 in Asia/Seoul). Both logs describe the same loaded path and diagnostic startup. No executable instructions were taken from the uploaded logs; they were treated as evidence.

## Exact event sequence

Line references below refer to `settings-diagnostics-32540-7ffae7930000.log`. Times are monotonic microseconds since diagnostic initialization.

| Log line | Time | Event |
| --- | ---: | --- |
| 8 | 16,963,291 | Both shortcut keys sampled down, focus true, no release publication. |
| 9 | 17,661,403 | F10 released while Shift remains down (`mask=1`); exactly one release publication, revision 1571 during the producer poll. |
| 10–16 | 17,662,729 onward | Present 1571 begins on swap chain `0x20ec3435470`. The same listener `2104d5d8a90` opens: `before=false after=true`, game tick 786, unchanged even revision 1572. Editor drawing runs, producing 5,748 vertices and 8,622 indices; overlay render returns success. |
| 17–23 | 17,663,779 onward | Present 1572 begins on a different swap chain `0x20ef3698810`, on the same thread. The same listener reads the same unchanged even revision 1572 and closes: `before=true after=false`. Its draw path now produces no geometry. |
| 24–25 | 17,663,896 / 17,667,413 | Present 1572 returns success before Present 1571 returns success. This nesting occurs after the first overlay-render result, consistent with reentry through the chained Present. |
| 26 | 17,674,475 | The next listener poll finally clears the release status. It is too late to prevent the second toggle. |
| 189 | 18,162,896 | Shift subsequently releases; this does not publish a second activation. |
| 333 | 77,233,479 | The only recorded focus-loss transition is much later, with neither shortcut key down and no activation. |

The open and close timestamps are 17,662,759 and 17,663,795 microseconds: **1.036 ms apart**. Both toggles have `revision_before=revision_after=1572` and game tick 786. Thus this is an actual state transition back to closed, rather than solely an invisible UI whose state remained open.

The trace captures 44 Present entries and 44 matching successful returns. No sampled overlay render returns an error, and no sampled pipeline-size skip occurs. The normal log contains two initialization-incomplete render errors at startup, about nine seconds before the shortcut, followed by successful UI initialization. Those startup errors do not explain the recorded close transition.

The two Present calls report the same HWND and dimensions but different swap-chain addresses and Windowed values. This is new evidence of a nested presentation path, not evidence of two Spell Wheel DLLs. These records do not identify which external component owns either swap chain, so they do not independently assign responsibility to ERSS, QuestPath, ME3 or their interaction. The separate crash during live FG switching remains outside this diagnosis.

## Connection to the source

In the diagnostic checkout, `src/keyboard/listener.rs:37–38` loads the shared boolean without consuming it. `src/keyboard/listener.rs:99–103` leaves the published value true until the next poll stores another value. `src/settings/display_settings.rs:95–107` toggles for each active read. `hudhook_fork/hudhook/src/hooks/dx12.rs:248–263` renders the overlay and then enters the chained Present, which can lead to the nested callback seen here.

The original source at the recorded base SHA has the same behavior: `src/keyboard/listener.rs:30–36,65–78` and `src/settings/display_settings.rs:95–105`. Diagnostic instrumentation did not add a second shortcut consumer or change release consumption. Timing changes introduced by instrumentation remain a general limitation, but the actual double-toggle is captured in this user's reproducing run.

## Proposed separate fix for review

Make the settings shortcut a game-input event with a monotonically increasing activation generation, and let each DisplaySettings instance remember the last generation it processed. The producer publishes one generation per valid physical chord release. Render updates process each generation once; nested or additional render calls observing the same generation do nothing. Preserve the shared listener behavior used by the other controls, or add a dedicated settings-shortcut event API, so wheel/controller consumers are not changed accidentally. Do not select or suppress an arbitrary swap chain merely to debounce this shortcut.

The settings-specific input state should also discard an in-progress chord on focus loss and wait for both keys to be released before arming after focus returns. This addresses the independently source-proven focus edge case without treating it as the trigger in this capture. Preserve activation when either chord key releases first, held-key stability, and the same shortcut's explicit close behavior. Define how multiple real activations between render updates are processed so event consumption does not silently discard deliberate actions.

Required automated validation for that future implementation:

- Replay the observed schedule: one release publication, outer render update, nested render update before the next poll. Visibility changes exactly once and stays open.
- Repeat render updates many times between polls; hold both keys across polls; release in both orders; verify one activation per chord.
- Close using a second deliberate chord and verify that no-presentation intervals do not lose pending input events.
- Lose/regain focus while keys are held; verify no phantom activation and correct re-arming after release.
- Keep original controller cycling and other listener behavior unchanged; run the existing locked MSVC build/checks.

Then perform the same affected-run game test with a candidate fix, followed by normal explicit close and controller/HDR smoke checks. Compilation alone cannot establish those runtime properties.

No additional configuration comparison is required to establish this double-consumption mechanism. A candidate fix still needs separate implementation authorization/review under the original handoff, and subsequent user acceptance in the affected setup. No production fix, external mod change, merge or release was performed while analyzing these logs.
