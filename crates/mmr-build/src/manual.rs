//! The manual: every setting (grouped like the app), the presets and the
//! tricks, as one self-contained HTML page made from the settings schema.

use mmr_settings::schema::{schema, SettingDef, SettingKind, LAYOUT_JSON};
use serde_json::{json, Value};

const TEMPLATE: &str = include_str!("manual.html");

/// How a value reads to a person ("On", "Casual", "3 items").
fn show(def: &SettingDef, layout: &Value, value: &Value) -> String {
    let option = |v: &str| -> String {
        layout["optionLabels"][&def.key][v]
            .as_str()
            .map(String::from)
            .or_else(|| def.options.iter().find(|o| o.value == v).map(|o| o.label.clone()))
            .unwrap_or_else(|| v.to_string())
    };
    match (def.kind, value) {
        (_, Value::Null) => "Automatic".into(),
        (SettingKind::Bool, Value::Bool(b)) => if *b { "On" } else { "Off" }.into(),
        (SettingKind::Enum, Value::String(s)) => option(s),
        (SettingKind::Flags, Value::Array(a)) if a.is_empty() => "None".into(),
        (SettingKind::Flags, Value::Array(a)) => a.iter().filter_map(Value::as_str).map(option).collect::<Vec<_>>().join(", "),
        (SettingKind::ItemList | SettingKind::Tricks, Value::Array(a)) => match a.len() {
            0 => "None".into(),
            1 => "1 entry".into(),
            n => format!("{n} entries"),
        },
        (_, Value::String(s)) if s.is_empty() => "None".into(),
        (_, Value::String(s)) => s.clone(),
        (_, Value::Number(n)) => n.to_string(),
        _ => "Custom".into(),
    }
}

/// The manual's data (also handy as JSON).
pub fn data() -> Value {
    let schema = schema();
    let layout: Value = serde_json::from_str(LAYOUT_JSON).expect("bundled layout");
    let presets = mmr_settings::presets();
    let label = |def: &SettingDef| layout["labels"][&def.key].as_str().map(String::from).unwrap_or_else(|| def.label.clone());
    let mut settings = serde_json::Map::new();
    for def in &schema.settings {
        let options: Vec<Value> = def
            .options
            .iter()
            .map(|o| json!({ "label": layout["optionLabels"][&def.key][&o.value].as_str().unwrap_or(&o.label), "description": o.description }))
            .collect();
        // What each preset sets it to, when that isn't the default.
        let in_presets: Vec<Value> = presets
            .iter()
            .filter_map(|p| p.settings.get(&def.key).filter(|v| **v != def.default).map(|v| json!([p.name, show(def, &layout, v)])))
            .collect();
        settings.insert(
            def.key.clone(),
            json!({
                "label": label(def),
                "description": def.description,
                "kind": def.kind,
                "options": options,
                "default": show(def, &layout, &def.default),
                "min": def.min,
                "max": def.max,
                "presets": in_presets,
                "cosmetic": mmr_settings::schema::is_cosmetic(&def.key),
            }),
        );
    }
    let mut tricks: Vec<Value> = Vec::new();
    for (name, logic) in [("Casual", "casual"), ("Glitched", "glitched")] {
        let Some(file) = mmr_logic::bundled(logic) else { continue };
        for e in &file.entries {
            let Some(t) = &e.trick else { continue };
            if let Some(existing) = tricks.iter_mut().find(|x| x["id"] == e.id.as_str()) {
                existing["logic"].as_array_mut().expect("logic list").push(name.into());
                continue;
            }
            tricks.push(json!({ "id": e.id, "description": t.description, "category": t.category, "url": t.url, "logic": [name] }));
        }
    }
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "sections": layout["sections"],
        "settings": settings,
        "presets": presets.iter().map(|p| json!({ "name": p.name, "description": p.description, "highlights": p.highlights })).collect::<Vec<_>>(),
        "tricks": tricks,
    })
}

/// The manual as one HTML page.
pub fn html() -> String {
    let data = serde_json::to_string(&data()).expect("manual data").replace("</", "<\\/");
    TEMPLATE.replacen("/*MANUAL_DATA*/null", &data, 1)
}

#[cfg(test)]
mod tests {
    #[test]
    fn manual_has_every_section_setting_and_trick() {
        let data = super::data();
        let settings = data["settings"].as_object().unwrap();
        for section in data["sections"].as_array().unwrap() {
            for group in section["groups"].as_array().unwrap() {
                for key in group["settings"].as_array().unwrap() {
                    assert!(settings.contains_key(key.as_str().unwrap()), "{key} is in the layout but not the manual");
                }
            }
        }
        assert!(data["tricks"].as_array().unwrap().len() > 300);
        let html = super::html();
        assert!(!html.contains("/*MANUAL_DATA*/") && html.contains("\"LogicMode\""));
    }
}
