# Spell Wheel controller release handoff

The user accepted all in-game checks for candidate `c1c27699566313d2b5d93bda8f0d3ac760da5579` on Elden Ring App Ver. **1.17.2** / Calibrations Ver. **1.17.3**, then approved updating GitHub and publishing. The accepted Windows x64/MSVC candidate was cross-built with locked dependencies and Rust nightly-2026-11-13. Its DLL SHA-256 is `d2cd3d90077986f5aab3faf3bef1ca5f02dce5ef8928a6c8e000ff3cd8f744df`.

Published as [controller-input-2d12dd2](https://github.com/BandarasX/SpellWheel/releases/tag/controller-input-2d12dd2) from source commit `2d12dd263095ec1d99fe547fd4b84d26c3f538f1`. The runtime source, dependencies, assets and tests match the accepted candidate. The native Windows x64/MSVC workflow passed 60 controller and panel/listener tests and 3 diagnostic transport tests before building and publishing the default-feature DLL. [Windows Actions run](https://github.com/BandarasX/SpellWheel/actions/runs/44939681257)

The public DLL is a separate build. Its SHA-256 is `592759619624097aab7d3425826729ea976a6c57181488c35d19635fe8c76d82` and its size is 7,821,824 bytes. All five public assets were downloaded. The tag, provenance, published checksums, AMD64 PE32+ header, and byte equality of the ZIP and standalone DLL were verified. The ZIP SHA-256 is `313fb2a023c12d6aaea0ac80c1862c6ebe96a9ead447193e89b4cf1c5bb850be`. [Build provenance](https://github.com/BandarasX/SpellWheel/releases/download/controller-input-2d12dd2/build-provenance.json) · [Release checksums](https://github.com/BandarasX/SpellWheel/releases/download/controller-input-2d12dd2/SHA256SUMS.txt)

## Acceptance and implementation

- `61678d3` fell through to vanilla controls when all six game input/HUD signature checks failed. `83a128d` restored hook retries and logged successful installation on executable product version **2.7.2.0**. The user reported that all gameplay checks passed except close audio.
- A changed close reached the sound guard on `83a128d`. `c1c2769` uses the version-matched native sound routine and a readable event table. The final user check passed on both wheels, covering changed close, silent unchanged/returned/cancelled close, and a short tap with one native cycle and click on release.
- The wheel state machine owns tap, hold, cancellation, and close selection. Native release requests pass through the equipment HUD update. Ranged precision aim and Telescope zoom own the press through release. The controller indicator snaps to its slot with an 8% angular margin per edge. The existing delay, radius, enablement and `switch_instantly` settings apply.
- The accepted candidate passed 60 tests in the controller/panel/listener harness and 3 diagnostic transport tests. Its default release build and all-features Windows x64/MSVC check passed. The final native-game test was supplied by the user.

Behavior, version-matched sources and acceptance steps are in [the controller candidate notes](spellwheel-controller-candidate.md). The prior [settings panel handoff](spellwheel-panel-chatgpt-handoff.md) records accepted panel behavior.

## Installation and rollback

Close Elden Ring, keep one active Spell Wheel DLL, and retain `spellwheel.toml`. Keep backup DLLs outside mod-loading folders. Set Frame Generation before launch. The live Frame Generation switch crash remains a separate issue.

For rollback, restore just the backed-up DLL or the prior accepted settings-panel DLL with SHA-256 `b564a8bbf477036a485cffd8a931d8b5a06f3dd89523f171d07e82f479b3022d`. [Settings-panel release](https://github.com/BandarasX/SpellWheel/releases/tag/settings-panel-56c8ed2)
