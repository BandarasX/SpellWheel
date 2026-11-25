<h1>Elden Ring Spell Wheel</h1>
Source code for the Spell Wheel mod, which allows you to quickly
switch spells without having to cycle through them one by one.

Nexus page: https://www.nexusmods.com/eldenring/mods/9636

<h2>HDR fork downloads and in-game settings</h2>

Download the Windows x64 DLL from the <a href="https://github.com/BandarasX/SpellWheel/releases/latest">latest GitHub Release</a>. Close the game before replacing the active DLL, preserve your existing <code>spellwheel.toml</code>, and keep backup DLLs outside mod-loading folders. Releases include source/build provenance and SHA-256 checksums.

The settings shortcut fix prevents one Left Shift + F10 release from toggling the editor twice when rendering is nested. The user-tested candidate stayed open and closed normally, with original controller spell/item cycling working. See the <a href="docs/spellwheel-settings-fix-chatgpt-handoff.md">complete investigation and handoff</a> for evidence and limitations.

Press **Left Shift + F10** by default to open the settings panel. Appearance, HDR, and Interface share a draft with the Advanced TOML editor. The panel can be moved and resized, and the wheel preview follows it when it opens or closes. See [settings, install instructions, and rollback](docs/spellwheel-panel.md).

**Interface → Wheel details** configures the highlighted spell and item card, availability messages, and counts on finite-item icons. See the [wheel details release handoff](docs/spellwheel-wheel-details-handoff.md).

<h2>Modded Spell Icons</h2>
<p>
In order to show icons of modded spells,
the user must specify a path to a directory.<br>
This path is relative to
<em>ELDEN RING/Game/mods/spellwheel/icons</em>.<br>
The directory must contain a path to mod/menu/hi, in which
must be <em>01_common.tpf.dcx</em> and <em>01_common.sblytbnd.dcx</em> files.<br>
These files are contain the compressed icon bytes and the icon atlas data respectively.
This method is supported by both Convergence and Reforged.
</p>
