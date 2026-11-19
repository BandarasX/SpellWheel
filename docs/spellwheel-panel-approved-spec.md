# Codex implementation handoff: approved Spell Wheel settings panel

Prepared 15 November 2026. Self-contained specification; no access to the planning chat, interactive illustrations, attachment-local paths or private tracker is required.

## Authorization and delivery boundary

The user approved this panel design and now explicitly requests continuing with implementation after the FG investigation. Implement it in the user's configured Codex Cloud repository/environment on an isolated branch, with tests and reviewable changes. No further generic permission question is needed for the scoped implementation. Do not merge, update master, publish a release, overwrite the installed game DLL/config, or modify external mods without separate user instructions. A diagnostic or candidate DLL/build workflow can be prepared for user acceptance. Do not automatically run a release-publishing workflow. Return a comprehensive handoff report for ChatGPT planning.

No implementation has been started merely by preparing this document.

## Baseline and essential references

- Fork: https://github.com/BandarasX/SpellWheel ; upstream https://github.com/notahackusator/SpellWheel (do not modify upstream).
- Master verified at preparation: `a7f6344eedad8dbaea04d41403df05f557a6bc57`. Inspect actual environment HEAD and applicable AGENTS.md before editing. Preserve intervening work; do not reset to this historical pin.
- Fixed shortcut implementation commit: `6b295323d1c329fbe8407a991398fd1b83cdb105`; release-source commit: `fccaa3b18b76f529a3831e4f4bb6f7fe4c4130f0`.
- Completed shortcut report: https://github.com/BandarasX/SpellWheel/blob/master/docs/spellwheel-settings-fix-chatgpt-handoff.md . Read this for architecture, regression tests and exact build instructions.
- Accepted normal release: https://github.com/BandarasX/SpellWheel/releases/tag/settings-shortcut-fix-fccaa3b . DLL SHA-256 `fe16ab221b2d2e27a47aa07bd3d21bc987f82aa1f0b5622ad52e4c9aac5a634f`.
- After the earlier diagnostic candidate passed, user also tested the normal release and said everything works fine. Preserve this baseline; this is not blanket validation of every configuration.
- Existing in-game TOML editor uses Left Shift + F10 and has Save and Reload icons. Extend its existing Dear ImGui/hudhook infrastructure rather than adding a web UI or unrelated overlay framework.

## FG investigation disposition (do not reopen as a prerequisite)

Codex's supplied findings report dated 15 November says the user reproduced the live ERSS Frame Generation-switch crash with spellwheel.dll removed. Spell Wheel is therefore NOT NECESSARY for that reported failure, subject to the user-reported removal. Cause remains unresolved; current evidence does not justify a Spell Wheel crash patch or blocking the panel.

Recorded environment in that report: ERSS 5.1.0, QuestPath 1.3.8, NVIDIA DLSS-FG without multi-frame generation; logs ME3 0.13.0 (989b522214e39c83d701f4182c4f0a86ad79a30a), RTX 4080 SUPER, executable 2.7.2.0 / ME3 game label 1.17.2.0 Worldwide; ME3 profile enables QuestPath and UltrawideUIFixes. Driver, exact binary hashes and toggle direction remain unknown. These are recorded values, not assumed current values for future tests.

Evidence nuance: the 17:00 FG-failure session and 17:25 normal-exit session both report breakpoint 0x80000003 at eldenring.exe+0xC59276. That signature is not specific to FG switching. ERSS reports of 0xC0000005 at QuestPath.dll+0x18108 belonged to the later normal-exit session with continued execution, not the supplied FG-failure session. Do not attribute fatal ownership from these records or merge the sessions.

No new controlled crash is required for this panel task. Establish desired FG state between launches and avoid live switching. A workaround is not a verified cure. Do not send external reports/messages or modify ERSS/QuestPath/ME3.

## Non-negotiable preservation

- Preserve HDR rendering, user TOML, and original controller spell/item cycling outside settings interaction.
- Preserve settings-shortcut activation generations/per-editor consumption and regression tests. The old bug processed the same release in nested Present callbacks and opened/closed the editor 1.036 ms apart. Do not return to a raw release boolean, frame-count debounce, swap-chain suppression or arbitrary rendering-frequency assumptions.
- Earlier duplicate wheels were caused by original and HDR DLLs both being loaded. Keep backups outside loading directories; do not presume duplicate instances are a new input-method feature.
- Do not claim the panel pauses Elden Ring. Do not change game-speed, saves, networking, anti-cheat, or external presentation settings.

## Approved composition and initial control inventory

Build one settings window with **Appearance**, **HDR**, **Interface** tabs, a visible active mode label, unsaved-state indicator, and a persistent footer. Do not add a Controls/Information tab yet; those are later enhancements. Do not add the sample-wheel preview studio in this version; reversible preview uses the existing wheel when appropriate and does not trigger gameplay actions.

Group related controls in independently collapsible sections. Allow multiple open; no nesting. Open the main section by default, remember expanded sections and tab choice during the editing session, and keep keyboard/controller focus on the header when a focused section collapses. Do not recreate drafts when tabs/sections change. Footer stays accessible; tab content may scroll on small displays.

### Appearance

| Section | User-facing control | Existing TOML key | Project reset value |
| --- | --- | --- | --- |
| Wheel layout | Style: Pretty / Simple | style | "pretty" |
| Wheel layout | Icon size | icon_scale_multiplier | 0.15 |
| Wheel layout | Wheel radius | radius_multiplier | 0.3 |
| Selection effect | Enlargement | highlight_expansion | 0.15 (display 15%) |
| Selection effect | Transition duration | highlight_time | 0.15 seconds |
| Labels and readability | Name placement: each icon / selected in center / hidden | item_names | "show" / "center" / "hide"; default "show" |
| Labels and readability | Font scale | font_scale_multiplier | 1.0 |
| Labels and readability | Text shadows | text_shadows | true |
| Colors and opacity | Icon tint | icon_tint | [1.0,1.0,1.0,1.0] |
| Colors and opacity | Icon-background tint | icon_background_tint | [1.0,1.0,1.0,1.0] |
| Colors and opacity | Label tint | name_tint | [1.0,1.0,1.0,1.0] |

Use native controls, readable labels/units and concise tooltips. Disable/annotate pretty-only controls when Simple is chosen without resetting their values. Percent displays need explicit conversion: e.g. 100% icon size represents base multiplier 0.15, not 1.0. Keep exact numeric entry where practical; do not silently clamp an existing config into slider display limits. Validate finite values and renderer-supported domains; derive safe limits from the renderer, document them and test boundary behavior. No imaginary HDR color simulation.

### HDR

- HDR output section: Enable HDR UI (`hdr_enabled`, project default true); UI brightness (`hdr_ui_brightness`, 400.0 nits).
- Color section: Saturation (`hdr_ui_saturation`, 1.0).
- These are PROJECT defaults deliberately chosen by the user. Current source defaults were false / 200.0 before this task. Preserve explicit existing user values on upgrade; do not rewrite a file or force 400 just because the panel opened. Use a clearly defined panel/new-config project defaults provider rather than accidentally altering unrelated compatibility fallbacks.
- Keep the panel readable through the working HDR pipeline. Preview brightness/saturation only through supported existing paths. If pipeline changes require apply/reload/restart, label and stage them accordingly; do not force resource recreation on each drag.
- Do not expose or change ERSS FG, screen mode or external HDR configuration.

### Interface

- Open/close settings: Function key F1-F12, optional SINGLE modifier None / Left Shift / Left Ctrl / Left Alt, with readable binding preview. Default Left Shift + F10.
- Reserve separate future wheel-input settings for a later task. No external-mod shortcut conflict detection, F13-F24, arbitrary-key recording or multiple simultaneous modifiers.
- Add documented TOML fields/schema as needed with backward-compatible parsing. Validate invalid values and retain a usable default. Preserve unknown keys/comments.

Other existing fields remain accessible in Advanced only. In particular, do not add, reset or redesign controller delays, selection radii, wheel key bindings, hook timing or force_rgba in this first graphical inventory. The user's controller delay 0.25 is a customization, NOT the source default 0.5. All-panel reset excludes these unmanaged fields and mod paths.

## Footer, modes, and editing semantics

Footer: Reset to Defaults… separated from **Save | Discard | Advanced mode**, with Advanced immediately beside Discard. In Advanced, it becomes **Graphical mode**. Advanced is a mode for the EXISTING TOML editor, not a tab or a separate configuration. Retain Reload icons in Advanced, clearly an explicit action (not a save); disable it when invalid draft/path changes make safe execution impossible rather than silently applying uncommitted settings.

Maintain saved settings/document, current draft/document, and effective runtime-preview state. Both modes operate on one draft:

- Graphical -> Advanced materializes current values while preserving comments, ordering and unrelated/unknown settings as far as feasible. Prefer editing a TOML document, not serializing a reduced struct over the whole file.
- Advanced -> Graphical validates syntax, known-key types, hotkey values and supported domains; on failure stay in Advanced and retain text with a precise error. Do not substitute defaults or lose text. Unknown keys alone are not an error.
- Valid visual draft changes can preview in memory; never write on every edit. Invalid text leaves the last valid preview intact. Do not preview gameplay/hook changes from Advanced in a way that mutates live input or requires unsafe reinitialization; stage those until Save/reload/restart as documented.
- Save validates and safely persists, then updates saved state and clears dirty state; keep panel open with success/error feedback. Return an actual Result from save. Use a same-directory temporary write/replace strategy appropriate for Windows; failed write leaves original file, draft, and active saved hotkey intact. Do not claim success from merely calling write.
- Discard restores the last successfully saved document/values and preview, retaining the panel. It must also restore Advanced edits and unsaved resets.
- Closing via shortcut/close action with dirty draft asks Save and close / Discard and close / Continue editing. Clean close needs no prompt. Invalid text cannot be saved; Continue/Discard remain available. Focus loss must not silently save, discard or toggle.
- Coordinate periodic settings disk rereads so they do not overwrite the draft or undo preview. Detect external disk changes during a dirty session and require explicit reload/overwrite choice; do not silently lose either side.

## Hotkey behavior

Generalize the existing fixed-chord generation/cursor implementation; do not replace it with per-render key checks.

- Exactly one activation per intentional chord, supporting either modifier-release order, no key-repeat toggles, and startup/focus-regain rearming after relevant keys are released.
- Modifier None means press/release of the selected function key, without unexpectedly activating when an extra Ctrl/Shift/Alt modifier is held. Require the selected modifier side, respecting Left vs Right.
- A new binding becomes active ONLY after successful Save, then release of old/new relevant keys. Changing dropdowns or editing TOML does not immediately change access. Saving a new binding must not immediately close/open the panel due to held input or stale generations. Handle reconfiguration without replaying old activations.
- A bad startup hotkey value must leave a documented usable recovery binding and diagnostic; avoid resetting unrelated settings. User can recover by editing TOML while game is closed.
- Settings hotkey remains usable in both modes, including dirty-close prompting, except while handling a modal in a way that would double-activate actions.

## Input ownership and visual accessibility

Use existing input infrastructure. While panel/modal captures interaction, the same navigation/confirmation input must not also select wheel entries or trigger gameplay actions. Release capture on close; original cycling resumes unchanged. Keep scope to menu input ownership, not a global controller redesign or new opening chord. If existing controller UI navigation is supported, ensure predictable focus, section expand/collapse and modal navigation; report limitations instead of silently claiming full controller UI support.

Keyboard/mouse controls must be fully usable, with visible focus, tooltips not the only source of critical information, and opaque enough surfaces/readable text in bright/dark HDR scenes. Size responsively and keep footer reachable at 3440x1440 and smaller windows; no assumptions about screenshot color accuracy. A browser mockup is not proof of DX12/HDR layout.

## Reset to Defaults: staged, scoped, reversible

1. Click Reset to Defaults… -> dialog with Current tab selected by default, or All panel settings, plus confirmation/cancel.
2. In Advanced mode, there is no current visible tab: offer All panel settings explicitly, never infer an invisible tab's scope.
3. Confirm loads only managed defaults into draft and safe preview, marks dirty and does not write. Cancel does nothing. Discard can undo the entire reset.
4. Full reset includes Appearance, HDR and Interface only, warns that HDR and hotkey can change, and uses the exact project table above. Preserve mods paths, unknown settings and unmanaged technical/controller fields. No automatic restart or external settings changes.
5. If Advanced TOML is invalid, block reset until it is fixed, or offer explicit Discard invalid edits and continue using last valid draft. Never silently replace invalid text or the whole file with defaults.
6. Reset button and mode/tab changes themselves must not trigger gameplay effects or lose focus/draft state.

## Tests and verification

Preserve and rerun the existing 13 shortcut/listener and 3 diagnostic-transport tests using current checked-in workflow/toolchain. Add meaningful coverage for:

- Shared-draft round-trip with comments, unknown keys, unmanaged fields, omitted keys and invalid TOML; no data loss across mode/tab changes.
- Save success, write failure and external-edit conflict; Discard, dirty close, and preview isolation from periodic reload.
- Project defaults and current-tab/all scopes, invalid-TOML reset cancellation/recovery; controller delay and mods paths unchanged.
- Every supported key/modifier including None; nested rendering, held keys, both release orders, focus loss/regain, queued events, Save/rebind and failed Save transitions. Existing fixed-chord regression must remain covered after generalization.
- Validation of finite numbers, percentages, current values beyond UI slider limits and pretty-only controls without destructive value changes.

Use current CI/build commands; known pinned nightly is 2026-11-13 and Windows x64/MSVC target. Native build/release workflow can publish on master pushes, so keep work isolated and do not trigger publication by accident. Build a candidate with provenance (source SHA, toolchain/features, DLL hash, artifact location). Cloud cannot run Elden Ring: list game acceptance as pending until user tests it.

Manual acceptance instructions should be concise and incremental: preserve TOML/backup; one active Spell Wheel DLL; open panel and check tabs/sections/modes; reversible appearance/HDR edits, Save/Discard, invalid TOML and reset; rebind/reopen; controller cycling outside menu; then verify with QuestPath + FG already configured before launch. Do not instruct live FG switching or repeat external crash testing. Include a rollback path to the accepted release without deleting settings.

## Deliverables and completion definition

Return reviewable branch/commit or PR as appropriate to the environment (no merge), change summary, actual automated check/build results, candidate installation and game test instructions, and a self-contained report for the planning chat. Distinguish implemented, automated-tested and user-game-tested behavior. Explain any unmet scope/limitations instead of silently dropping requirements or claiming success from compilation. Document new keys, project defaults, recovery, reload/restart requirements and save/error semantics.

Finish only after implementing the scoped panel and completing feasible checks, or identifying a specific blocker. Production publication and final in-game acceptance are separate steps.
