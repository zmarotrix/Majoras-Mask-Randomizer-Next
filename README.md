# Majora's Mask Randomizer Next

A randomizer for *The Legend of Zelda: Majora's Mask* (N64, USA). It shuffles
items, entrances, music, enemies and more, and makes sure every seed can be
finished. MMR Next is a rewrite of the original C# randomizer in Rust, with a
desktop app for Windows, macOS and Linux and a version that runs entirely in
your web browser.

You need your own Majora's Mask (USA) ROM (`.z64`, `.n64` or `.v64`). It is
never uploaded anywhere: in the browser version too, it stays on your device.

<img width="1182" height="932" alt="image" src="https://github.com/user-attachments/assets/57587755-1348-433f-a225-d6c784517611" />

<img width="1182" height="932" alt="image" src="https://github.com/user-attachments/assets/63e714d1-af21-480c-800a-1688a5dfa1dc" />


## What you get with each seed

- **The randomized ROM**, or a small **patch** (`.mmrn`) others can apply to
  their own ROM.
- **A tracker** (`.html`): a spoiler-free checklist for playing the seed. It
  works out what you can reach with what you've found (time of day
  included), keeps your progress, and reveals answers one at a time.
- **A spoiler log** with the playthrough, hints, entrances and every check.
- **Hash icons**: everyone on the same seed sees the same five icons on the
  file select screen.

The same seed and settings make the same ROM on every system.

## Getting it

Download the latest release from the [Releases page](https://github.com/zmarotrix/Majoras-Mask-Randomizer-Next/releases):

| File | What |
| --- | --- |
| `MMRNext-<version>-windows-x64-setup.exe` | Windows installer (or the `.msi`) |
| `MMRNext-<version>-windows-x64-portable.exe` | Windows, no install needed |
| `MMRNext-<version>-macos-universal.dmg` | macOS (Intel and Apple Silicon) |
| `MMRNext-<version>-linux-x64.AppImage` / `.deb` / `.rpm` | Linux |
| `MMRNext-<version>-web.zip` | The browser version (static files; host them at the root of any web server) |
| `mmr-cli-<version>-<system>` | The command-line tool |

The apps aren't code-signed yet. On **macOS**, the first time: right-click
the app, choose **Open**, then **Open** again (or run
`xattr -dr com.apple.quarantine "/Applications/Majora's Mask Randomizer.app"`).
On **Windows**, SmartScreen may warn: choose **More info → Run anyway**. On
**Linux**, make the AppImage executable (`chmod +x`) and run it.

## Building it yourself

You need [Rust](https://rustup.rs) and [Node.js](https://nodejs.org).

| Windows | macOS / Linux | To |
| --- | --- | --- |
| `Launch MMR Next.bat` | `./launch.sh` | run the desktop app in development mode |
| `Launch MMR Next (Web).bat` | `./launch-web.sh` | build the browser version and open it at `localhost:4173` |
| `Build Release.bat` | `./build-release.sh` | build everything this system can into `releases/` |

Each system builds its own installers (Tauri can't cross-compile them); the
Release workflow builds all three on GitHub. `./build-release.sh --universal`
makes one macOS app for both Intel and Apple Silicon. On Linux, install
WebKitGTK first (Debian/Ubuntu:
`sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`).

Or by hand:

```sh
cd app && npm install
npm run tauri dev        # desktop app, development mode
npm run tauri build      # desktop app + installers (target/release/bundle)
npm run build:web        # browser version (app/dist); needs: rustup target add wasm32-unknown-unknown
cargo test --workspace   # tests
cargo run -p mmr-cli -- --help
```

Tests that need a ROM look for `../Zelda - Majora's Mask.z64` or the path in
`MMR_TEST_ROM`, and skip themselves without one.

### Releasing

Write the release notes in `docs/releases/<tag>.md`, then either push a
version tag (e.g. `v3.0.0`) or run **Actions → Release → Run workflow** on
GitHub. GitHub's servers build Windows, macOS and Linux with the same script
as `Build Release.bat`, and put everything in a draft release to check and
publish.

## Command line

```sh
mmr generate --preset modern --seed 123 --rom MM.z64 --tracker-out seed.html
mmr apply-patch seed.mmrn --rom MM.z64
mmr stats --preset modern --count 500        # how seeds turn out, without a ROM
mmr manual                                   # every setting and trick, as a web page
mmr settings presets
mmr logic check casual
```

## Repository layout

| Path | What |
| --- | --- |
| `crates/mmr-data` | Game data (items, regions, categories), embedded from `data/` |
| `crates/mmr-logic` | Logic files: format, region logic, conversion from older versions, validation |
| `crates/mmr-settings` | Settings schema, presets, validation, settings strings |
| `crates/mmr-randomizer` | Item placement, entrances, prices, hints analysis |
| `crates/mmr-build` | Builds the ROM; spoiler log, tracker and manual |
| `crates/mmr-rom` | ROM checks, byte order, file system, patches |
| `crates/mmr-web` | The randomizer for the browser (WebAssembly) |
| `crates/mmr-cli` | The `mmr` command-line tool |
| `app/` | The app (Svelte UI; Tauri for desktop) |
| `data/` | Game data, settings schema, presets, bundled logic and music |
| `tools/` | Scripts that extracted `data/` from the original C# source, and a comparison harness |

See [ROADMAP.md](ROADMAP.md) for what's done and what's next.

## Logic files

Logic is plain JSON. Each entry says what is needed to reach a location,
obtain an event, or satisfy a helper:

```json
{
  "id": "ItemBow",
  "requires": ["AreaWoodFallTempleAccess", "WFT 2nd Floor Access"],
  "any_of": [["Any Sword"], ["OtherArrow"], ["MaskDeku", "Magic Meter"]]
}
```

Everything in `requires`, plus everything in at least one `any_of` group.
Logic files from older versions of the randomizer open directly, and the app
can save in the older format too. The app's logic editor shows logic as
regions connected by exits.

## License

GPL-3.0, like the original randomizer.
