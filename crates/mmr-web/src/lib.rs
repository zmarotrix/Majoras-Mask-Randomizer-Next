//! The randomizer for the browser.
//!
//! A plain WebAssembly interface, so no binding generator is needed: the
//! page copies inputs into wasm memory, calls one command, and copies the
//! outputs back. Every command takes JSON plus any number of byte blobs (a
//! ROM, a patch) and gives back JSON plus blobs (the ROM, the tracker...).
//!
//! ```text
//! p = mmr_alloc(n); write bytes at p; mmr_push_blob(p, n)     (per blob)
//! status = mmr_call(name, name_len, json, json_len)           (0 = ok)
//! mmr_out_json_ptr/len, mmr_out_blob_count, mmr_out_blob_ptr/len(i)
//! ```
//!
//! On error the output JSON is `{"error": "..."}`. `web/mmr.js` wraps this.

use serde_json::{json, Value};
use std::cell::RefCell;

type Blobs = Vec<Vec<u8>>;
type Output = Result<(Value, Blobs), String>;

thread_local! {
    static INPUT: RefCell<Blobs> = const { RefCell::new(Vec::new()) };
    static OUT_JSON: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUT_BLOBS: RefCell<Blobs> = const { RefCell::new(Vec::new()) };
}

/// Run one command (also usable natively, e.g. in tests).
pub fn call(name: &str, input: &Value, blobs: Blobs) -> Output {
    match name {
        "version" => Ok((json!(env!("CARGO_PKG_VERSION")), vec![])),
        "settings_check" => {
            let values = values_of(input)?;
            Ok((json!({ "issues": mmr_settings::validate(&values), "inactive": mmr_settings::inactive(&values) }), vec![]))
        }
        "settings_encode" => {
            let values = values_of(input)?;
            let cosmetics = input["includeCosmetics"].as_bool().unwrap_or(false);
            Ok((json!(mmr_settings::encode_settings_string(&values, cosmetics)), vec![]))
        }
        "settings_decode" => {
            let string = input["string"].as_str().unwrap_or("");
            let values = mmr_settings::decode_settings_string(string).map_err(|e| e.to_string())?;
            Ok((serde_json::to_value(values).map_err(|e| e.to_string())?, vec![]))
        }
        // Settings files: MMR Next's own format, or an older settings.json.
        "settings_open" => {
            let text = input["text"].as_str().unwrap_or("");
            let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("Not a valid settings file: {e}"))?;
            if value["format"] == SETTINGS_FORMAT {
                let partial = value["settings"].as_object().cloned().unwrap_or_default();
                return Ok((json!({ "values": mmr_settings::with_defaults(&partial), "skipped": [], "legacy": false }), vec![]));
            }
            let imported = mmr_settings::legacy::import_settings_file(text).map_err(|e| e.to_string())?;
            Ok((json!({ "values": imported.values, "skipped": imported.skipped, "legacy": true }), vec![]))
        }
        "settings_save" => {
            let values = values_of(input)?;
            let doc = json!({ "format": SETTINGS_FORMAT, "version": 1, "settings": mmr_settings::changed(&values) });
            Ok((Value::Null, vec![(serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())? + "\n").into_bytes()]))
        }
        "rom_check" => {
            let mut rom = blob(blobs, 0, "the ROM")?;
            Ok((rom_check(&mut rom), vec![]))
        }
        "generate" => generate(input, blobs),
        "apply_patch" => {
            let mut it = blobs.into_iter();
            let (Some(mut clean), Some(patch)) = (it.next(), it.next()) else { return Err("Choose your ROM and the patch.".into()) };
            mmr_rom::prepare(&mut clean).map_err(|e| e.to_string())?;
            let (rom, info) = mmr_rom::patch::apply(&clean, &patch).map_err(|e| e.to_string())?;
            Ok((json!({ "info": serde_json::from_slice::<Value>(info).unwrap_or(Value::Null) }), vec![rom]))
        }
        "manual" => Ok((Value::Null, vec![mmr_build::manual::html().into_bytes()])),
        other => Err(format!("unknown command {other}")),
    }
}

const SETTINGS_FORMAT: &str = "mmr-settings";

fn values_of(input: &Value) -> Result<mmr_settings::Values, String> {
    let partial: mmr_settings::Values = serde_json::from_value(input["values"].clone()).map_err(|e| format!("settings: {e}"))?;
    Ok(mmr_settings::with_defaults(&partial))
}

fn blob(blobs: Blobs, i: usize, what: &str) -> Result<Vec<u8>, String> {
    blobs.into_iter().nth(i).ok_or_else(|| format!("missing {what}"))
}

fn rom_check(rom: &mut [u8]) -> Value {
    match mmr_rom::prepare(rom) {
        Ok(info) => {
            let message = if info.original_order != mmr_rom::ByteOrder::BigEndian {
                "Majora's Mask (USA). This ROM was in a different byte order; it will be converted automatically."
            } else {
                "Majora's Mask (USA), ready to randomize."
            };
            json!({ "ok": true, "message": message, "info": info })
        }
        Err(e) => json!({ "ok": false, "message": e.to_string(), "info": null }),
    }
}

/// Like the desktop app's `generate`, with files as blobs: the JSON lists
/// `files: [{kind, name}]` in the same order as the blobs.
fn generate(input: &Value, blobs: Blobs) -> Output {
    let mut rom = blob(blobs, 0, "the ROM")?;
    mmr_rom::prepare(&mut rom).map_err(|e| e.to_string())?;
    let values = values_of(input)?;
    let logic = match values.get("LogicMode").and_then(Value::as_str).unwrap_or("Casual") {
        "Casual" | "Vanilla" | "NoLogic" => mmr_logic::bundled("casual").expect("bundled logic"),
        "Glitched" => mmr_logic::bundled("glitched").expect("bundled logic"),
        "UserLogic" => {
            let text = input["userLogic"].as_str().ok_or("Choose a custom logic file first (Logic: Custom logic file).")?;
            mmr_logic::load(text).map_err(|e| e.to_string())?.0
        }
        other => return Err(format!("The \"{other}\" logic mode isn't supported.")),
    };
    let style = match values.get("ItemPlacement").and_then(Value::as_str) {
        Some("Bespoke") => mmr_randomizer::Style::Bespoke,
        Some("Classic") => mmr_randomizer::Style::Classic,
        _ => mmr_randomizer::Style::Random,
    };
    // The page supplies a random seed (the browser has no OS randomness here).
    let seed = mmr_randomizer::Seed::parse(input["seed"].as_str().ok_or("missing seed")?);
    let mut options = mmr_build::Options::default();
    let mut notes: Vec<String> = Vec::new();
    for (i, text) in input["plando"].as_array().into_iter().flatten().filter_map(Value::as_str).enumerate() {
        options.plando.extend(mmr_randomizer::plando::parse(text).map_err(|e| format!("item plando file {}: {e}", i + 1))?);
        notes.push("Item plando applied".into());
    }
    let built = mmr_build::seed::build_seed(&rom, &values, &logic, seed, style, &options).map_err(|e| e.to_string())?;

    let outputs = &input["outputs"];
    let want = |k: &str| outputs[k].as_bool().unwrap_or(false);
    let stem = format!("MMRN-{}", built.outcome.seed);
    let hash: String = built.hash.iter().map(|b| format!("{b:02X}")).collect();
    let mut files: Vec<Value> = Vec::new();
    let mut out: Blobs = Vec::new();
    let mut add = |kind: &str, name: String, data: Vec<u8>| {
        files.push(json!({ "kind": kind, "name": name }));
        out.push(data);
    };
    if want("patch") {
        let info = json!({
            "app": "MMR Next",
            "version": env!("CARGO_PKG_VERSION"),
            "seed": built.outcome.seed.to_string(),
            "settings": mmr_settings::encode_settings_string(&values, false),
            "hash": hash,
            "hashIcons": built.hash_icons.iter().map(|i| i.name).collect::<Vec<_>>(),
        });
        add("patch", format!("{stem}.mmrn"), mmr_rom::patch::create(&rom, &built.rom, info.to_string().as_bytes()));
    }
    if want("spoiler") || want("tracker") {
        let info = mmr_build::spoiler::BuildInfo { hash: &built.hash, icons: &built.hash_icons, music: &built.music, item_icons: &built.item_icons };
        let s = mmr_build::spoiler::build(&built.outcome, &built.plan, &values, &logic, info);
        if want("spoiler") {
            add("spoiler", format!("{stem}.spoiler.txt"), mmr_build::spoiler::text(&s).into_bytes());
        }
        if want("tracker") {
            add("tracker", format!("{stem}.tracker.html"), mmr_build::spoiler::html(&s).into_bytes());
        }
    }
    if want("hashIcons") || values.get("DrawHash").and_then(Value::as_bool) == Some(true) {
        add("hashIcons", format!("{stem}.png"), mmr_build::hash_icons::png(&built.hash_icons));
    }
    if want("rom") {
        add("rom", format!("{stem}.z64"), built.rom);
    }
    let icons: Vec<Value> = built.hash_icons.iter().map(|i| json!({ "name": i.name, "png": format!("data:image/png;base64,{}", mmr_build::spoiler::base64(&mmr_build::hash_icons::icon_png(i))) })).collect();
    Ok((json!({ "seed": built.outcome.seed.to_string(), "hash": hash, "hashIcons": icons, "files": files, "notes": notes }), out))
}

// ---- The WebAssembly interface -----------------------------------------------

/// Reserve `len` bytes for the page to write into.
#[no_mangle]
pub extern "C" fn mmr_alloc(len: usize) -> *mut u8 {
    let mut v: Vec<u8> = Vec::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Give back memory from `mmr_alloc` that wasn't pushed as a blob.
///
/// # Safety
/// `ptr` and `len` must come from one `mmr_alloc` call.
#[no_mangle]
pub unsafe extern "C" fn mmr_free(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

/// Hand a blob (from `mmr_alloc`, filled) to the next `mmr_call`.
///
/// # Safety
/// `ptr` and `len` must come from one `mmr_alloc` call, filled with `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn mmr_push_blob(ptr: *mut u8, len: usize) {
    let v = Vec::from_raw_parts(ptr, len, len.max(1));
    INPUT.with(|i| i.borrow_mut().push(v));
}

/// Run a command. The name and JSON are UTF-8, from `mmr_alloc` (freed
/// here). Returns 0 on success, 1 on error.
///
/// # Safety
/// Pointers and lengths must come from `mmr_alloc` calls, filled.
#[no_mangle]
pub unsafe extern "C" fn mmr_call(name: *mut u8, name_len: usize, input: *mut u8, input_len: usize) -> u32 {
    let name = Vec::from_raw_parts(name, name_len, name_len.max(1));
    let input = Vec::from_raw_parts(input, input_len, input_len.max(1));
    let blobs = INPUT.with(|i| std::mem::take(&mut *i.borrow_mut()));
    let result = std::str::from_utf8(&name)
        .map_err(|e| e.to_string())
        .and_then(|name| {
            let json: Value = if input.is_empty() { Value::Null } else { serde_json::from_slice(&input).map_err(|e| e.to_string())? };
            call(name, &json, blobs)
        });
    let (status, json, blobs) = match result {
        Ok((json, blobs)) => (0, json, blobs),
        Err(e) => (1, json!({ "error": e }), Vec::new()),
    };
    OUT_JSON.with(|o| *o.borrow_mut() = serde_json::to_vec(&json).unwrap_or_default());
    OUT_BLOBS.with(|o| *o.borrow_mut() = blobs);
    status
}

#[no_mangle]
pub extern "C" fn mmr_out_json_ptr() -> *const u8 {
    OUT_JSON.with(|o| o.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn mmr_out_json_len() -> usize {
    OUT_JSON.with(|o| o.borrow().len())
}

#[no_mangle]
pub extern "C" fn mmr_out_blob_count() -> usize {
    OUT_BLOBS.with(|o| o.borrow().len())
}

#[no_mangle]
pub extern "C" fn mmr_out_blob_ptr(i: usize) -> *const u8 {
    OUT_BLOBS.with(|o| o.borrow().get(i).map_or(std::ptr::null(), |b| b.as_ptr()))
}

#[no_mangle]
pub extern "C" fn mmr_out_blob_len(i: usize) -> usize {
    OUT_BLOBS.with(|o| o.borrow().get(i).map_or(0, Vec::len))
}

/// Drop the last outputs (they can be large).
#[no_mangle]
pub extern "C" fn mmr_out_clear() {
    OUT_JSON.with(|o| *o.borrow_mut() = Vec::new());
    OUT_BLOBS.with(|o| *o.borrow_mut() = Vec::new());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        let (s, _) = call("settings_encode", &json!({ "values": { "LogicMode": "Glitched" } }), vec![]).unwrap();
        let (v, _) = call("settings_decode", &json!({ "string": s }), vec![]).unwrap();
        assert_eq!(v["LogicMode"], "Glitched");
    }

    #[test]
    fn errors_are_messages() {
        assert!(call("rom_check", &Value::Null, vec![]).is_err());
        assert!(call("nope", &Value::Null, vec![]).is_err());
        let (check, _) = call("rom_check", &Value::Null, vec![vec![0; 64]]).unwrap();
        assert_eq!(check["ok"], false);
    }
}
