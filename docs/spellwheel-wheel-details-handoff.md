# Spell Wheel HDR wheel details release handoff

The wheel details release adds a configurable card for the highlighted spell or quick item and a count on each finite-item icon. The user completed the previous candidate checks, reproduced a false **Cannot summon** warning in an offline session, and confirmed the fix in candidate `f6e20d8ff1955a39ce11a25fb311d3536893f866`. Release binaries are built separately by the native Windows x64/MSVC workflow. [GitHub releases](https://github.com/BandarasX/SpellWheel/releases)

## Install and roll back

1. Close Elden Ring. Back up your active `spellwheel.dll` and `spellwheel.toml` outside all mod-loading folders.
2. Replace only the active DLL. Keep your TOML, the rest of your mod files, and one active Spell Wheel DLL. Set Frame Generation before launching.
3. Open the settings panel with **Left Shift + F10** by default, then open **Interface → Wheel details**.

To roll back, restore the backed-up DLL or download the previous public controller release. The controller DLL SHA-256 is `592759619624097aab7d3425826729ea976a6c57181488c35d19635fe8c76d82`. Keep your TOML. Older builds ignore wheel-details keys. [Previous controller release](https://github.com/BandarasX/SpellWheel/releases/tag/controller-input-2d12dd2)

To verify a download before installing, run `Get-FileHash .\spellwheel.dll -Algorithm SHA256` in PowerShell and compare it with `SHA256SUMS.txt` from the same release. The release ZIP contains the DLL, installation instructions, this handoff, build provenance, a DLL checksum, and the Atkinson Hyperlegible font license.

## What players see

- **Spell wheel.** The card shows initial cast FP adjusted by active sorcery or incantation multipliers. Fractional costs round up once at the end. **Insufficient FP** appears when current FP is strictly below the displayed cost. Each INT, FAI, and ARC requirement shows its own state in muted green, muted red, or neutral when the current value is unavailable.
- **Quick-item wheel.** The card can show FP and HP costs from the goods row and scaling grades from the linked virtual weapon. Resource warnings compare current FP or HP with the displayed cost. Applicable warnings share one centered line. Finite items show their counts in the lower-right corner of their icons, including zero if the game retains the depleted slot. Reusable items have no count.
- **Spirit Ashes and Puppets.** The pre-summon location check uses the current monument ID. The card for the active summon suppresses its own **Cannot summon** warning. Resource warnings remain independent.
- **Placement and preview.** The card follows the accepted highlight, including before an on-close selection commits. Its name is independent of wheel names. It can appear above or below, with adjustable background transparency and a separate gold border toggle. The settings preview uses the selected quick item when opened with no wheel visible. Opening settings with a wheel already visible retains that wheel and its highlight.

## Settings reference

The settings apply through the existing preview, save, reload, and Interface reset controls. A TOML missing these keys uses the defaults below.

| TOML key | Values | Default |
| --- | --- | --- |
| `wheel_details_wheels` | `"both"`, `"spell"`, `"item"`, `"off"` | `"both"` |
| `wheel_details_show_name` | Boolean | `true` |
| `wheel_details_fp_cost` | Boolean, spell FP | `true` |
| `wheel_details_stat_requirements` | Boolean | `true` |
| `wheel_details_item_quantity` | Boolean, icon counts | `true` |
| `wheel_details_quantity_font_scale` | `0.25` to `3.0`, equivalent to 25% to 300% | `1.0` |
| `wheel_details_availability` | Boolean | `true` |
| `wheel_details_position` | `"below"` or `"above"` | `"below"` |
| `wheel_details_background_transparency` | `0.0` to `1.0` | `0.06` |
| `wheel_details_gold_border` | Boolean | `true` |

The quantity size slider spans 50% to 200%, with exact entry from 25% to 300%. At 100% it uses the original icon-relative size and fits the number inside the icon. At 100% background transparency the fill disappears. The gold border follows its own toggle. Between the card and wheel, the nominal gap is 24 pixels and can fall to 4 pixels when space is tight. [Full settings guide](https://github.com/BandarasX/SpellWheel/blob/master/docs/spellwheel-panel.md)

## Implementation and investigation

- `src/wheel_details_data.rs` reads spell, item, inventory, resource, and summon state in the game tick. Snapshots match the wheel type, slot index, and parameter ID. `src/wheel_details.rs` applies visibility settings and builds the warning display.
- `src/rendering/render_hook.rs` connects the snapshot to the accepted highlight and settings preview. `src/rendering/details_card.rs` draws the card in the existing HDR and SDR overlay. `src/rendering/display_item.rs` draws icon counts after each icon. The existing settings defaults and save behavior are in `src/settings`.
- Diagnostics at `34e69f3` established the offline false positive. The game reported no current monument outside the area, current monument `10000104` when the HUD icon appeared before summoning, and active monument `10000104` after a successful summon. The activation-range flag was false in the usable pre-summon state. The final candidate uses the current monument ID for the pre-summon location check and still identifies the active summon by goods ID. This change does not depend on which launcher starts the game. [Pinned summon manager fields](https://github.com/vswarte/fromsoftware-rs/blob/09a4208ac26af4ae07b08c5273ee0e89faf97cb8/crates/eldenring/src/cs/world_chr_man.rs)
- Cost coverage is the initial cast for spells and the goods row cost for items. Charged, continuous, and replacement spells can have additional native costs beyond the displayed initial cast. Individual scripted summoning rules are outside the cases confirmed in the offline tests. [Spell cost data](https://github.com/EldenRingDatabase/erdb/blob/e2028a6e044b920a471388fb4e1c468b31b64350/src/erdb/table/spells.py)

## Acceptance and provenance

| Candidate | Acceptance or change |
| --- | --- |
| `11f1081` | Initial card, settings, preview, and item count behavior passed the in-game checklist. |
| `fd3cfce` | Centered card, live FP modifiers, individual stat colors, item scaling, card spacing, and appearance controls. The user found discounted FP could be one point low. |
| `88febc2` | Upward FP rounding, smaller minimum gap, and independent card-name setting. The user confirmed all checks passed. |
| `3ccfdba` | Item warnings and finite-item counts on icons. The user confirmed the remaining issue was the summon warning. |
| `001f5ff` | Quantity font scale passed. The user reported the active summon behaved correctly and identified the remaining false warning before summoning. |
| `f6e20d8` | Current monument ID corrected the pre-summon warning. The user confirmed it works and approved publication. |

The accepted candidate's default-feature Windows x64/MSVC cross-build passed 73 harness tests and 3 diagnostic transport tests, plus the Windows all-features check. Its DLL SHA-256 is `768ac9a038af5e724122907177a1caa4de1641137aef558b037e2b57d1f0b6e6`. The published DLL is a separate native build. Use its own `build-provenance.json` and `SHA256SUMS.txt` to identify it. Release packaging uses Rust nightly-2026-11-13 and locked dependencies. The Actions workflow tests before building or publishing.

## Focused smoke test

1. Install just the DLL and keep your existing TOML. On both wheels, check short tap, hold, highlight changes, and normal and cancelled close.
2. Compare the displayed spell cost against the game menu with and without Primal Glintstone Blade. Check the warning with FP just below and exactly equal to the displayed cost.
3. Check a finite item count, a reusable item, and the quantity font-size setting. Verify card name, Above/Below placement, preview, Save, and Interface reset.
4. In a fresh summon area, verify **Cannot summon** appears outside the area and clears when the monument icon appears inside it. After summoning, the active Ash or Puppet should have no summon warning. A different summon item should still show the warning while the spirit is active.
