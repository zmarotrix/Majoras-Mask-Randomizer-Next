//! Tauri commands: a thin layer exposing the core crates to the UI.
//!
//! The UI talks to the core only through these commands (see
//! `app/src/lib/backend.ts`). A browser build provides the same calls via
//! wasm, so keep file-system access here and out of the core crates.

use mmr_logic::{eval, validate, LogicFile, SourceFormat};
use mmr_settings::Values;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::PathBuf;

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn read_text(path: &str) -> CmdResult<String> {
    std::fs::read_to_string(path).map_err(|e| format!("Couldn't read {path}: {e}"))
}

fn write_text(path: &str, text: &str) -> CmdResult<()> {
    std::fs::write(path, text).map_err(|e| format!("Couldn't write {path}: {e}"))
}

// ---- Game data & settings ------------------------------------------------

#[tauri::command]
fn game_data() -> Value {
    let g = mmr_data::game();
    json!({ "items": g.items, "regions": g.regions, "categories": g.categories })
}

#[tauri::command]
fn settings_schema() -> Value {
    json!({
        "settings": mmr_settings::schema::schema().settings,
        "categories": mmr_settings::schema::CATEGORIES,
        "defaults": mmr_settings::defaults(),
        "layout": serde_json::from_str::<Value>(mmr_settings::schema::LAYOUT_JSON).expect("layout is valid"),
    })
}

#[tauri::command]
fn presets() -> Vec<mmr_settings::Preset> {
    mmr_settings::presets()
}

#[derive(Serialize)]
struct SettingsCheck {
    issues: Vec<mmr_settings::Issue>,
    inactive: Vec<mmr_settings::Inactive>,
}

#[tauri::command]
fn settings_check(values: Values) -> SettingsCheck {
    SettingsCheck { issues: mmr_settings::validate(&values), inactive: mmr_settings::inactive(&values) }
}

#[tauri::command]
fn settings_encode(values: Values, include_cosmetics: bool) -> String {
    mmr_settings::encode_settings_string(&values, include_cosmetics)
}

#[tauri::command]
fn settings_decode(string: String) -> CmdResult<Values> {
    mmr_settings::decode_settings_string(&string).map_err(err)
}

const SETTINGS_FORMAT: &str = "mmr-settings";

#[derive(Serialize)]
struct LoadedSettings {
    values: Values,
    /// Legacy settings that couldn't be carried over.
    skipped: Vec<String>,
    legacy: bool,
}

/// Open a settings file: MMR Next format, or an MMR 2.0 settings.json.
#[tauri::command]
fn settings_open(path: String) -> CmdResult<LoadedSettings> {
    let text = read_text(&path)?;
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("Not a valid settings file: {e}"))?;
    if value.get("format").and_then(Value::as_str) == Some(SETTINGS_FORMAT) {
        let partial = value.get("settings").and_then(Value::as_object).cloned().unwrap_or_default();
        return Ok(LoadedSettings { values: mmr_settings::with_defaults(&partial), skipped: vec![], legacy: false });
    }
    let imported = mmr_settings::legacy::import_settings_file(&text).map_err(err)?;
    Ok(LoadedSettings { values: imported.values, skipped: imported.skipped, legacy: true })
}

#[tauri::command]
fn settings_save(path: String, values: Values) -> CmdResult<()> {
    let doc = json!({ "format": SETTINGS_FORMAT, "version": 1, "settings": mmr_settings::changed(&values) });
    write_text(&path, &(serde_json::to_string_pretty(&doc).map_err(err)? + "\n"))
}

// ---- Logic ----------------------------------------------------------------

#[derive(Serialize)]
struct BundledLogic {
    id: &'static str,
    name: &'static str,
}

#[tauri::command]
fn logic_bundled_list() -> Vec<BundledLogic> {
    mmr_data::BUNDLED_LOGIC.iter().map(|(id, name, _)| BundledLogic { id, name }).collect()
}

#[derive(Serialize)]
struct OpenedLogic {
    file: LogicFile,
    format: SourceFormat,
}

/// The tricks in a bundled logic (for the tricks picker).
#[tauri::command]
async fn logic_tricks(id: String) -> CmdResult<Vec<mmr_logic::Entry>> {
    let file = mmr_logic::bundled(&id).ok_or_else(|| format!("No bundled logic called {id}"))?;
    Ok(file.entries.into_iter().filter(|e| e.trick.is_some()).collect())
}

/// Open a bundled logic (`bundled:<id>`) or a logic file by path.
#[tauri::command]
async fn logic_open(source: String) -> CmdResult<OpenedLogic> {
    if let Some(id) = source.strip_prefix("bundled:") {
        let file = mmr_logic::bundled(id).ok_or_else(|| format!("No bundled logic called {id}"))?;
        return Ok(OpenedLogic { file, format: SourceFormat::Current });
    }
    let (file, format) = mmr_logic::load(&read_text(&source)?).map_err(err)?;
    Ok(OpenedLogic { file, format })
}

#[tauri::command]
async fn logic_save(path: String, file: LogicFile, legacy: bool) -> CmdResult<()> {
    let text = if legacy {
        let schema = mmr_settings::schema::schema();
        let enum_type = |k: &str| schema.get(k).and_then(|d| d.enum_type.clone());
        serde_json::to_string_pretty(&mmr_logic::legacy::to_legacy(&file, &enum_type)).map_err(err)?
    } else {
        mmr_logic::to_json(&file)
    };
    write_text(&path, &(text + "\n"))
}

#[tauri::command]
async fn logic_validate(file: LogicFile) -> Vec<validate::Issue> {
    let known: HashSet<String> = mmr_settings::schema::schema().settings.iter().map(|s| s.key.clone()).collect();
    validate::validate(&file, Some(&known))
}

#[tauri::command]
async fn logic_evaluate(file: LogicFile, scenario: eval::Scenario) -> Vec<String> {
    eval::Evaluator::new(&file).evaluate(&scenario).satisfied.into_iter().collect()
}

#[tauri::command]
async fn logic_playthrough(file: LogicFile, scenario: eval::Scenario) -> eval::Playthrough {
    eval::Evaluator::new(&file).vanilla_playthrough(&scenario)
}

// ---- Region logic (the logic editor) ---------------------------------------

#[derive(Serialize)]
struct Conversion {
    /// The converted logic behaves exactly like the original.
    equivalent: bool,
    scenarios: usize,
    entries: usize,
    regions: usize,
    /// Checks whose rule names no place (they go in their area's region).
    unmapped: usize,
    /// Entries that behave differently (only if not equivalent).
    mismatched: Vec<String>,
}

#[derive(Serialize)]
struct OpenedRegions {
    logic: mmr_logic::region::RegionLogic,
    /// "regions", "entries" (MMR Next entry format) or "old" (older MMR).
    format: &'static str,
    /// Present when the file was converted to regions while opening.
    conversion: Option<Conversion>,
}

/// Convert entry-style logic to regions and prove it behaves the same.
fn convert_to_regions(old: &LogicFile) -> (mmr_logic::region::RegionLogic, Conversion) {
    let (regions, report) = mmr_logic::regionize::to_regions_auto(old);
    let new = regions.compile();
    let game = mmr_data::game();
    let items: Vec<String> = game.items.iter().filter(|i| i.name.is_some()).map(|i| i.id.clone()).collect();
    let tricks: Vec<String> = old.entries.iter().filter(|e| e.trick.is_some()).map(|e| e.id.clone()).collect();
    let presets: Vec<_> = mmr_settings::presets().iter().map(|p| mmr_settings::apply_preset(p, None)).collect();
    let cases = mmr_logic::equivalence::scenarios(400, 0xC0FFEE, &items, &tricks, &presets);
    let pairs: Vec<(String, String)> =
        old.entries.iter().map(|e| (e.id.clone(), report.replaced.get(&e.id).cloned().unwrap_or_else(|| e.id.clone()))).collect();
    let result = mmr_logic::equivalence::compare_pairs(old, &new, &pairs, &cases);
    let mut mismatched: Vec<String> = result.mismatches.iter().map(|m| m.id.clone()).collect();
    mismatched.sort();
    mismatched.dedup();
    let conversion = Conversion {
        equivalent: result.is_equivalent(),
        scenarios: result.scenarios,
        entries: result.compared_ids,
        regions: report.regions,
        unmapped: report.unmapped_locations.len(),
        mismatched,
    };
    (regions, conversion)
}

/// Open logic for the editor: bundled (`bundled:<id>`) or a file. Older
/// formats are converted to regions and checked.
#[tauri::command]
async fn logic_open_regions(source: String) -> CmdResult<OpenedRegions> {
    if let Some(id) = source.strip_prefix("bundled:") {
        let logic = mmr_logic::bundled_regions(id).ok_or_else(|| format!("No bundled logic called {id}"))?;
        return Ok(OpenedRegions { logic, format: "regions", conversion: None });
    }
    let text = read_text(&source)?;
    if let Some(logic) = mmr_logic::parse_regions(&text).map_err(err)? {
        return Ok(OpenedRegions { logic, format: "regions", conversion: None });
    }
    let (file, format) = mmr_logic::load(&text).map_err(err)?;
    let (logic, conversion) = convert_to_regions(&file);
    let format = if format == SourceFormat::Legacy { "old" } else { "entries" };
    Ok(OpenedRegions { logic, format, conversion: Some(conversion) })
}

/// Save region logic, or export it in the older MMR format.
#[tauri::command]
async fn logic_save_regions(path: String, logic: mmr_logic::region::RegionLogic, old_format: bool) -> CmdResult<()> {
    let text = if old_format {
        let schema = mmr_settings::schema::schema();
        let enum_type = |k: &str| schema.get(k).and_then(|d| d.enum_type.clone());
        serde_json::to_string_pretty(&mmr_logic::legacy::to_legacy(&logic.compile(), &enum_type)).map_err(err)?
    } else {
        serde_json::to_string_pretty(&logic).map_err(err)?
    };
    write_text(&path, &(text + "\n"))
}

/// Problems in region logic: structure (exits to missing regions, regions
/// nothing leads to) and everything the entry validator finds.
#[tauri::command]
async fn logic_check_regions(logic: mmr_logic::region::RegionLogic) -> Vec<validate::Issue> {
    use mmr_logic::region::region_entry_id;
    use std::collections::BTreeSet;
    use validate::Severity::{Error, Warning};
    let mut issues = Vec::new();
    let names: BTreeSet<&str> = logic.regions.iter().map(|r| r.name.as_str()).collect();
    let mut seen = BTreeSet::new();
    let mut entered: BTreeSet<&str> = BTreeSet::from([logic.start.as_str()]);
    for r in &logic.regions {
        if !seen.insert(r.name.as_str()) {
            issues.push(validate::Issue::new(Error, "duplicate_region", &region_entry_id(&r.name), format!("There are two regions called \"{}\"", r.name)));
        }
        for e in &r.exits {
            entered.insert(e.to.as_str());
            if !names.contains(e.to.as_str()) {
                issues.push(validate::Issue::new(Error, "unknown_region", &region_entry_id(&r.name), format!("Exit to \"{}\", which isn't a region", e.to)));
            }
        }
    }
    for r in &logic.regions {
        if !entered.contains(r.name.as_str()) {
            issues.push(validate::Issue::new(Warning, "no_way_in", &region_entry_id(&r.name), format!("Nothing leads to \"{}\"", r.name)));
        }
    }
    let known: HashSet<String> = mmr_settings::schema::schema().settings.iter().map(|s| s.key.clone()).collect();
    issues.extend(validate::validate(&logic.compile(), Some(&known)));
    issues
}

/// What's reachable with a scenario (items, tricks, settings): entry ids,
/// including "Region: <name>" for regions.
#[tauri::command]
async fn logic_reach_regions(logic: mmr_logic::region::RegionLogic, scenario: eval::Scenario) -> Vec<String> {
    eval::Evaluator::new(&logic.compile()).evaluate(&scenario).satisfied.into_iter().collect()
}

// ---- ROM ------------------------------------------------------------------

#[derive(Serialize)]
struct RomCheck {
    ok: bool,
    message: String,
    info: Option<mmr_rom::RomInfo>,
}

#[tauri::command]
fn rom_check(path: String) -> CmdResult<RomCheck> {
    let mut bytes = std::fs::read(PathBuf::from(&path)).map_err(|e| format!("Couldn't read the ROM: {e}"))?;
    Ok(match mmr_rom::prepare(&mut bytes) {
        Ok(info) => {
            let converted = info.original_order != mmr_rom::ByteOrder::BigEndian;
            let message = if converted {
                "Majora's Mask (USA). This ROM was in a different byte order; it will be converted automatically.".into()
            } else {
                "Majora's Mask (USA), ready to randomize.".into()
            };
            RomCheck { ok: true, message, info: Some(info) }
        }
        Err(e) => RomCheck { ok: false, message: e.to_string(), info: None },
    })
}

/// What generating writes.
#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Outputs {
    rom: bool,
    patch: bool,
    spoiler: bool,
    tracker: bool,
    hash_icons: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HashIconOut {
    name: String,
    /// `data:image/png;base64,...`
    png: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Generated {
    rom_path: Option<String>,
    patch_path: Option<String>,
    spoiler_path: Option<String>,
    tracker_path: Option<String>,
    /// Every file written.
    files: Vec<String>,
    seed: String,
    /// The build hash, hex.
    hash: String,
    hash_icons: Vec<HashIconOut>,
    millis: u128,
    /// Anything worth knowing about how the seed was made.
    notes: Vec<String>,
}

fn icons_out(icons: &[mmr_build::hash_icons::HashIcon]) -> Vec<HashIconOut> {
    icons
        .iter()
        .map(|i| HashIconOut { name: i.name.to_string(), png: format!("data:image/png;base64,{}", base64(&mmr_build::hash_icons::icon_png(i))) })
        .collect()
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            out.push(if i <= chunk.len() { T[(n >> (18 - 6 * i)) as usize & 63] as char } else { '=' });
        }
    }
    out
}

/// `base.ext` next to the chosen output (whose own extension is dropped).
fn beside(out: &str, suffix: &str) -> PathBuf {
    let p = PathBuf::from(out);
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "seed".into());
    p.with_file_name(format!("{stem}{suffix}"))
}

fn write_out(path: &PathBuf, data: &[u8], what: &str, files: &mut Vec<String>) -> CmdResult<String> {
    std::fs::write(path, data).map_err(|e| format!("Couldn't write the {what}: {e}"))?;
    let s = path.to_string_lossy().into_owned();
    files.push(s.clone());
    Ok(s)
}

/// Randomize and build. Writes what `outputs` asks for, next to `out_path`.
#[tauri::command]
async fn generate(app: tauri::AppHandle, rom_path: String, out_path: String, values: Values, seed: String, outputs: Outputs) -> CmdResult<Generated> {
    let started = std::time::Instant::now();
    let mut bytes = std::fs::read(&rom_path).map_err(|e| format!("Couldn't read the ROM: {e}"))?;
    mmr_rom::prepare(&mut bytes).map_err(err)?;
    let values = mmr_settings::with_defaults(&values);

    let mode = values.get("LogicMode").and_then(Value::as_str).unwrap_or("Casual");
    let logic = match mode {
        // Vanilla and No Logic still use the casual logic for the playthrough.
        "Casual" | "Vanilla" | "NoLogic" => mmr_logic::bundled("casual").expect("bundled logic"),
        "Glitched" => mmr_logic::bundled("glitched").expect("bundled logic"),
        "UserLogic" => {
            let path = values.get("UserLogicFileName").and_then(Value::as_str).unwrap_or("");
            if path.is_empty() {
                return Err("Choose a custom logic file first (Logic: Custom logic file).".into());
            }
            mmr_logic::load(&read_text(path)?).map_err(err)?.0
        }
        other => return Err(format!("The \"{other}\" logic mode isn't supported.")),
    };
    let style = match values.get("ItemPlacement").and_then(Value::as_str) {
        Some("Bespoke") => mmr_randomizer::Style::Bespoke,
        Some("Classic") => mmr_randomizer::Style::Classic,
        _ => mmr_randomizer::Style::Random,
    };
    let seed = if seed.trim().is_empty() { mmr_randomizer::Seed::random() } else { mmr_randomizer::Seed::parse(&seed) };

    // Extra music packs: a "music" folder next to the app or in its data folder.
    let mut options = mmr_build::Options::default();
    if values.get("Cosmetic.Music").and_then(Value::as_str) == Some("Random") {
        for dir in music_folders(&app) {
            if let Ok(files) = mmr_build::music::read_folder(&dir) {
                options.music.extend(files);
            }
        }
    }
    // Item plando: *ItemPlando.json next to the app or in its data folder's "plando" folder.
    let mut notes = Vec::new();
    for dir in plando_folders(&app) {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.to_string_lossy().to_ascii_lowercase().ends_with("itemplando.json")).collect();
        paths.sort();
        for path in paths {
            let combos = mmr_randomizer::plando::parse(&read_text(&path.to_string_lossy())?).map_err(|e| format!("{}: {e}", path.display()))?;
            notes.push(format!("Item plando from {}", path.file_name().unwrap_or_default().to_string_lossy()));
            options.plando.extend(combos);
        }
    }
    let built = mmr_build::seed::build_seed(&bytes, &values, &logic, seed, style, &options).map_err(|e| e.to_string())?;

    let mut files = Vec::new();
    let hash: String = built.hash.iter().map(|b| format!("{b:02X}")).collect();
    let rom_out = if outputs.rom { Some(write_out(&PathBuf::from(&out_path), &built.rom, "ROM", &mut files)?) } else { None };
    let patch_out = if outputs.patch {
        let info = json!({
            "app": "MMR Next",
            "version": env!("CARGO_PKG_VERSION"),
            "seed": built.outcome.seed.to_string(),
            "settings": mmr_settings::encode_settings_string(&values, false),
            "hash": hash,
            "hashIcons": built.hash_icons.iter().map(|i| i.name).collect::<Vec<_>>(),
        });
        let patch = mmr_rom::patch::create(&bytes, &built.rom, info.to_string().as_bytes());
        Some(write_out(&beside(&out_path, ".mmrn"), &patch, "patch", &mut files)?)
    } else {
        None
    };
    let (spoiler_out, tracker_out) = if outputs.spoiler || outputs.tracker {
        let s = mmr_build::spoiler::build(&built.outcome, &built.plan, &values, &logic, mmr_build::spoiler::BuildInfo { hash: &built.hash, icons: &built.hash_icons, music: &built.music, item_icons: &built.item_icons });
        let text = if outputs.spoiler { Some(write_out(&beside(&out_path, ".spoiler.txt"), mmr_build::spoiler::text(&s).as_bytes(), "spoiler log", &mut files)?) } else { None };
        let html = if outputs.tracker { Some(write_out(&beside(&out_path, ".tracker.html"), mmr_build::spoiler::html(&s).as_bytes(), "tracker", &mut files)?) } else { None };
        (text, html)
    } else {
        (None, None)
    };
    if outputs.hash_icons || values.get("DrawHash").and_then(Value::as_bool) == Some(true) {
        write_out(&beside(&out_path, ".png"), &mmr_build::hash_icons::png(&built.hash_icons), "hash icons", &mut files)?;
    }
    if files.is_empty() {
        notes.push("Nothing was written: every output is switched off.".into());
    }
    Ok(Generated {
        rom_path: rom_out,
        patch_path: patch_out,
        spoiler_path: spoiler_out,
        tracker_path: tracker_out,
        files,
        seed: built.outcome.seed.to_string(),
        hash,
        hash_icons: icons_out(&built.hash_icons),
        millis: started.elapsed().as_millis(),
        notes,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Patched {
    rom_path: String,
    /// What the patch says about its seed (JSON).
    info: Value,
}

/// Open a file the app wrote (e.g. the tracker) with its usual program.
#[tauri::command]
async fn open_file(app: tauri::AppHandle, path: String) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_path(path, None::<&str>).map_err(err)
}

/// Write the manual (every setting, preset and trick) and open it in the
/// browser.
#[tauri::command]
async fn open_manual(app: tauri::AppHandle) -> CmdResult<()> {
    use tauri::Manager;
    use tauri_plugin_opener::OpenerExt;
    let dir = app.path().app_cache_dir().map_err(err)?;
    std::fs::create_dir_all(&dir).map_err(err)?;
    let path = dir.join("MMR Next Manual.html");
    std::fs::write(&path, mmr_build::manual::html()).map_err(err)?;
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(err)
}

/// Rebuild a randomized ROM from a clean ROM and an MMR Next patch.
#[tauri::command]
async fn apply_patch(rom_path: String, patch_path: String, out_path: String) -> CmdResult<Patched> {
    let mut clean = std::fs::read(&rom_path).map_err(|e| format!("Couldn't read the ROM: {e}"))?;
    mmr_rom::prepare(&mut clean).map_err(err)?;
    let patch = std::fs::read(&patch_path).map_err(|e| format!("Couldn't read the patch: {e}"))?;
    let (rom, info) = mmr_rom::patch::apply(&clean, &patch).map_err(err)?;
    std::fs::write(&out_path, &rom).map_err(|e| format!("Couldn't write the ROM: {e}"))?;
    Ok(Patched { rom_path: out_path, info: serde_json::from_slice(info).unwrap_or(Value::Null) })
}

/// Where players can drop item plando files.
fn plando_folders(app: &tauri::AppHandle) -> Vec<PathBuf> {
    use tauri::Manager;
    let mut dirs = Vec::new();
    if let Some(exe) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        dirs.push(exe);
    }
    if let Ok(data) = app.path().app_data_dir() {
        dirs.push(data.join("plando"));
    }
    dirs
}

/// Where players can drop extra music packs.
fn music_folders(app: &tauri::AppHandle) -> Vec<PathBuf> {
    use tauri::Manager;
    let mut dirs = Vec::new();
    if let Some(exe) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("music"))) {
        dirs.push(exe);
    }
    if let Ok(data) = app.path().app_data_dir() {
        dirs.push(data.join("music"));
    }
    dirs
}

// ---- Windows ---------------------------------------------------------------

/// Open the logic editor in its own resizable window, or bring it to the
/// front if it's open.
// Async: on Windows, creating a window from a command that runs on the
// main thread deadlocks.
#[tauri::command]
async fn open_logic_editor(app: tauri::AppHandle) -> CmdResult<()> {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
    if let Some(w) = app.get_webview_window("logic-editor") {
        let _ = w.unminimize();
        let _ = w.show();
        return w.set_focus().map_err(err);
    }
    // The page knows it's the editor from the window's label. It opens at a
    // normal size; maximize or full-screen it as you like.
    WebviewWindowBuilder::new(&app, "logic-editor", WebviewUrl::App("index.html".into()))
        .title("Majora's Mask Randomizer Next: Logic editor")
        .inner_size(1440.0, 900.0)
        .min_inner_size(1000.0, 680.0)
        .center()
        .build()
        .map(|_| ())
        .map_err(err)
}

// ---- Preferences ---------------------------------------------------------

/// The preferences file: `preferences.json` in the app's config folder
/// (e.g. %APPDATA%\<app id> on Windows). Written immediately on every
/// change, so nothing is lost if the app is closed abruptly.
fn prefs_path(app: &tauri::AppHandle) -> CmdResult<PathBuf> {
    use tauri::Manager;
    let dir = app.path().app_config_dir().map_err(err)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't create {}: {e}", dir.display()))?;
    Ok(dir.join("preferences.json"))
}

#[tauri::command]
fn prefs_load(app: tauri::AppHandle) -> Value {
    prefs_path(&app)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}))
}

#[tauri::command]
fn prefs_save(app: tauri::AppHandle, prefs: Value) -> CmdResult<()> {
    let path = prefs_path(&app)?;
    // Write to a temporary file first so a crash can't leave half a file.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&prefs).map_err(err)?).map_err(err)?;
    std::fs::rename(&tmp, &path).map_err(err)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Development: MMR_OPEN_EDITOR=1 opens the logic editor at launch.
            if std::env::var("MMR_OPEN_EDITOR").is_ok_and(|v| v == "1") {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = open_logic_editor(handle).await {
                        eprintln!("couldn't open the logic editor: {e}");
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            game_data,
            settings_schema,
            presets,
            settings_check,
            settings_encode,
            settings_decode,
            settings_open,
            settings_save,
            logic_bundled_list,
            logic_open,
            logic_save,
            logic_validate,
            logic_evaluate,
            logic_playthrough,
            rom_check,
            generate,
            apply_patch,
            open_file,
            open_manual,
            prefs_load,
            prefs_save,
            open_logic_editor,
            logic_tricks,
            logic_open_regions,
            logic_save_regions,
            logic_check_regions,
            logic_reach_regions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the app");
}
