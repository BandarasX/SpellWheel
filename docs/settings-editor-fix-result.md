# Settings shortcut candidate: game test passed

> Historical investigation/test record. See [the final ChatGPT handoff](spellwheel-settings-fix-chatgpt-handoff.md) for the completed publication status.

The user reports that the editor stayed open for the test duration, closed normally with the shortcut, and original controller spell/item cycling worked normally. The supplied logs corroborate the shortcut fix: one activation opens the editor, the nested Present does not close it, and a second deliberate activation closes it about 29.88 seconds later.

This validates the disappearing-editor fix in the tested setup. It does not establish general compatibility with every external-mod configuration or validate the separate live-FG-switch crash.

## Build identity

- Base HEAD: `76556aa96c52842a46ed672814dbb771546d784c`.
- Embedded candidate source-patch SHA-256: `565960d0f69b609549a132470a72155112d6e025931970965bc95983f3b1584c`, matching the delivered candidate's build provenance.
- Delivered candidate DLL SHA-256: `9819d8a6049966e86696e9c86d855609ed8aa6927465050a8f997a7fb5c3b4b5`. Its installed byte hash was not independently supplied; the running module reports the matching build label.
- Runtime PID: `5440`; module base: `7ffae8840000`; one editor identity: `listener=1`.
- The test continues the previously reported QuestPath-enabled, FG-on reproduction procedure. Exact external-mod versions and launcher configuration are not encoded in these logs.

## Runtime evidence

References are to `settings-diagnostics-5440-7ffae8840000.log`.

| Evidence | Observation |
| --- | --- |
| Line 2 | Running module identifies the candidate source patch, rather than the earlier diagnostic-only build. |
| Lines 9, 12 | Generation 1 is published and applied once: `before=false after=true activations=1`. Open timestamp: 20,320,986 microseconds. |
| Lines 17–25 | A second swap chain enters nested Present on the same thread, as in the failing trace. It draws the editor with `shown=true` without applying generation 1 again. Both Present calls return success. |
| Open interval | All 44 sampled editor-draw records between the two actions have `shown=true`. There is no intervening close activation. Sampling covers the armed diagnostic window, not every frame of the full interval; the user's observation confirms continued visible display. |
| Lines 332, 335 | Generation 2 is published and applied once: `before=true after=false activations=1`. Close timestamp: 50,201,126 microseconds. |
| Lines 343–351 | The nested callback leaves the editor closed rather than toggling it open again. |
| Final flush, line 655 | 630 attempted events, `dropped=0 capped=false`. Exactly two published generations and exactly two processed editor activations are captured. |

The open-to-close interval is **29.880140 seconds**. There are 88 sampled Present entries with matching successful returns, no sampled overlay-render errors, and no recorded pipeline-size skips. A later focus-loss sample publishes no activation. The ordinary log again contains two initialization-incomplete render errors before UI initialization; they are startup observations, not failures during the successful open/close test.

Controller cycling success is user-observed evidence, rather than a conclusion inferred from render logs. The user did not separately report a new HDR appearance check, focus-regain/held-key runtime test, or both release orders. Automated tests cover input-state edge cases; broader runtime claims are not made.

## Evidence hashes

- `settings-diagnostics-5440-7ffae8840000.log`: `d2d966b8a9cac8249afd94e3540bbe064dd89671b433da9bdbf9ab7931a76984`.
- `spellwheel.log`: `8e8ec82a36ecbd8d1ad6730f6241d7f364aa5ad1524fb0e8c95326fd8426e0bd`.

## Status

The observed double-toggle is fixed in the user-tested candidate, with normal explicit close and original controller cycling confirmed for this run. No further code change or rebuild was needed to analyze these results. The existing 16 passing automated tests, Windows x64 builds, source patches and candidate archive remain unchanged. The build provenance's original `runtime_validation=NOT RUN` describes build time; this report supplies the subsequent user-run evidence.

The fix remains an uncommitted, reviewable working-tree change on `investigate/settings-editor-diagnostics`; no merge or release was performed. The graphical settings panel and the separate crash when switching FG remain outside this work.

The user can remove `spellwheel-settings-diagnostics.enable` beside the active DLL and restart to disable diagnostic file logging while keeping the candidate fix. No TOML change or additional DLL replacement is required for that step.
