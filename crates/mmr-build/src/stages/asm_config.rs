//! Cosmetic config for the payload, written after the hash is known
//! (`WriteAsmConfig`: D-pad, HUD colours, world colours, music, hash).

use crate::color::{translate_hsv, Hsv, Rgb};
use crate::dotnet_random::DotNetRandom;
use crate::{BuildError, Builder};
use serde_json::Value;
use std::collections::HashMap;

const WHITE: Rgb = Rgb { r: 255, g: 255, b: 255 };

const fn c(r: u8, g: u8, b: u8) -> Rgb {
    Rgb { r, g, b }
}

/// HUD colours in config order, with defaults (`HudColors`).
const HUD_COLORS: [(&str, Rgb); 53] = [
    ("ButtonA", c(0x64, 0xC8, 0xFF)),
    ("ButtonB", c(0x64, 0xFF, 0x78)),
    ("ButtonC", c(0xFF, 0xF0, 0x00)),
    ("ButtonStart", c(0xFF, 0x82, 0x3C)),
    ("ClockEmblem", c(0x00, 0xAA, 0x64)),
    ("ClockEmblemInverted1", c(0x64, 0xCD, 0xFF)),
    ("ClockEmblemInverted2", c(0x00, 0x9B, 0xFF)),
    ("ClockEmblemSun", c(0xFF, 0xFF, 0x6E)),
    ("ClockMoon", c(0xFF, 0xFF, 0x37)),
    ("ClockSun", c(0xFF, 0x64, 0x6E)),
    ("Heart", c(0xFF, 0x46, 0x32)),
    ("HeartDD", c(0xC8, 0x00, 0x00)),
    ("Magic", c(0x00, 0xC8, 0x00)),
    ("MagicInf", c(0x00, 0x00, 0xC8)),
    ("Map", c(0x00, 0xFF, 0xFF)),
    ("MapEntranceCursor", c(0xC8, 0x00, 0x00)),
    ("MapPlayerCursor", c(0xC8, 0xFF, 0x00)),
    ("RupeeIcon1", c(0xC8, 0xFF, 0x64)),
    ("RupeeIcon2", c(0xAA, 0xAA, 0xFF)),
    ("RupeeIcon3", c(0xFF, 0x69, 0x69)),
    ("ButtonIconA", c(0x50, 0x5A, 0xFF)),
    ("ButtonIconB", c(0x46, 0xFF, 0x50)),
    ("ButtonIconC", c(0xFF, 0xFF, 0x32)),
    ("MenuAInner1", c(0x64, 0x96, 0xFF)),
    ("MenuAInner2", c(0x64, 0xFF, 0xFF)),
    ("MenuAOuter1", c(0x00, 0x00, 0x64)),
    ("MenuAOuter2", c(0x00, 0x96, 0xFF)),
    ("MenuCInner1", c(0xFF, 0xFF, 0x00)),
    ("MenuCInner2", c(0xFF, 0xFF, 0x00)),
    ("MenuCOuter1", c(0x00, 0x00, 0x00)),
    ("MenuCOuter2", c(0xFF, 0xA0, 0x00)),
    ("NoteA1", c(0x50, 0x96, 0xFF)),
    ("NoteA2", c(0x64, 0xC8, 0xFF)),
    ("NoteAShadow1", c(0x0A, 0x0A, 0x0A)),
    ("NoteAShadow2", c(0x32, 0x32, 0xFF)),
    ("NoteC1", c(0xFF, 0xFF, 0x32)),
    ("NoteC2", c(0xFF, 0xFF, 0xB4)),
    ("NoteCShadow1", c(0x0A, 0x0A, 0x0A)),
    ("NoteCShadow2", c(0x6E, 0x6E, 0x32)),
    ("PauseTitleA", c(0x00, 0x64, 0xFF)),
    ("PauseTitleC", c(0xFF, 0x96, 0x00)),
    ("Prompt1", c(0x00, 0x50, 0xC8)),
    ("Prompt2", c(0x32, 0x82, 0xFF)),
    ("PromptGlow", c(0x00, 0x82, 0xFF)),
    ("ScoreLines", c(0xFF, 0x00, 0x00)),
    ("ScoreNote", c(0xFF, 0x64, 0x00)),
    ("DPad", c(0x80, 0x80, 0x80)),
    ("MenuBorder1", c(0xB4, 0xB4, 0x78)),
    ("MenuBorder2", c(0x96, 0x8C, 0x5A)),
    ("MenuSubtitleText", c(0xFF, 0xC8, 0x00)),
    ("ShopCursor1", c(0x00, 0x00, 0xFF)),
    ("ShopCursor2", c(0x00, 0x50, 0xFF)),
    ("RupeeIcon4", c(0xA0, 0x60, 0xFF)),
];

/// `HudColorPresets.StandardPairs`.
const STANDARD_PAIRS: [(Rgb, Rgb); 7] = [
    (c(0xFF, 0x33, 0x00), c(0xCC, 0x00, 0x66)),
    (c(0xFF, 0x99, 0x00), c(0xFF, 0xC2, 0x66)),
    (c(0xFF, 0xFF, 0x00), c(0xFF, 0xFF, 0x66)),
    (c(0x00, 0x66, 0x00), c(0x33, 0xCC, 0x33)),
    (c(0x33, 0x66, 0xCC), c(0x33, 0xCC, 0xFF)),
    (c(0x99, 0x00, 0xFF), c(0x99, 0x66, 0xFF)),
    (c(0xFF, 0x66, 0xD9), c(0xFF, 0xB3, 0xEC)),
];

/// World colours by name, with defaults (`WorldColors`).
const WORLD_COLORS: [(&str, Rgb); 46] = [
    ("DekuDustInner", c(0xFF, 0xFF, 0x37)),
    ("DekuDustOuter", c(0x64, 0x32, 0x00)),
    ("DekuSparklesInner", c(0xFF, 0xC8, 0xC8)),
    ("DekuSparklesOuter", c(0xFF, 0xFF, 0x00)),
    ("FierceDeitySparklesInner", c(0x64, 0xFF, 0xFF)),
    ("FierceDeitySparklesOuter", c(0x00, 0x64, 0x64)),
    ("GoronPunchEnergyEnv1", c(0xFF, 0x00, 0x00)),
    ("GoronPunchEnergyEnv2", c(0xFF, 0x00, 0x00)),
    ("GoronPunchEnergyPrim", c(0xFF, 0xC8, 0x32)),
    ("GoronRollInnerEnergyEnv", c(0x9B, 0x00, 0x00)),
    ("GoronRollInnerEnergyPrim", c(0xFF, 0x9B, 0x00)),
    ("GoronRollOuterEnergyEnv1", c(0x64, 0x00, 0x00)),
    ("GoronRollOuterEnergyEnv2", c(0xC8, 0x00, 0x00)),
    ("GoronRollOuterEnergyPrim1", c(0xFF, 0x00, 0x00)),
    ("GoronRollOuterEnergyPrim2", c(0xFF, 0x9B, 0x00)),
    ("SwordBeamDamageEnv", c(0x00, 0xFF, 0xFF)),
    ("SwordBeamEnergyEnv", c(0x00, 0x64, 0xFF)),
    ("SwordBeamEnergyPrim", c(0xAA, 0xFF, 0xFF)),
    ("SwordChargeBlueEnergyEnv", c(0x00, 0x64, 0xFF)),
    ("SwordChargeBlueEnergyPrim", c(0xAA, 0xFF, 0xFF)),
    ("SwordChargeRedEnergyEnv", c(0xFF, 0x64, 0x00)),
    ("SwordChargeRedEnergyPrim", c(0xFF, 0xFF, 0xAA)),
    ("SwordChargeSparksBlue", c(0x00, 0x00, 0xFF)),
    ("SwordChargeSparksRed", c(0xFF, 0x00, 0x00)),
    ("SwordEnergyBlueEnv1", c(0x00, 0x00, 0xFF)),
    ("SwordEnergyBlueEnv2", c(0x00, 0x64, 0xFF)),
    ("SwordEnergyBluePrim", c(0xAA, 0xFF, 0xFF)),
    ("SwordEnergyRedEnv1", c(0xFF, 0x00, 0x00)),
    ("SwordEnergyRedEnv2", c(0xFF, 0x64, 0x00)),
    ("SwordEnergyRedPrim", c(0xFF, 0xFF, 0xAA)),
    ("ZoraEnergyEnv1", c(0x00, 0x00, 0x64)),
    ("ZoraEnergyEnv2", c(0x00, 0x96, 0xFF)),
    ("ZoraEnergyPrim1", c(0x00, 0x96, 0xFF)),
    ("ZoraEnergyPrim2", c(0xAA, 0xFF, 0xFF)),
    ("BlueBubble", c(0x00, 0x00, 0xFF)),
    ("FireArrowEffectEnv", c(0xFF, 0x00, 0x00)),
    ("FireArrowEffectPrim", c(0xFF, 0xC8, 0x00)),
    ("IceArrowEffectEnv", c(0x00, 0x00, 0xFF)),
    ("IceArrowEffectPrim", c(0xAA, 0xFF, 0xFF)),
    ("LightArrowEffectEnv", c(0xFF, 0xFF, 0x00)),
    ("LightArrowEffectPrim", c(0xFF, 0xFF, 0xAA)),
    ("FierceDeityTunic", c(0xBD, 0xB5, 0xAD)),
    ("GoronTunic", c(0x1E, 0x69, 0x1B)),
    ("ZoraTunic", c(0x1E, 0x69, 0x1B)),
    ("DekuTunic", c(0x1E, 0x69, 0x1B)),
    ("HumanTunic", c(0x1E, 0x69, 0x1B)),
];

/// The world colours in the config struct's order (`StructColors`).
const WORLD_STRUCT: [&str; 25] = [
    "GoronPunchEnergyEnv1",
    "GoronRollInnerEnergyEnv",
    "SwordChargeBlueEnergyEnv",
    "SwordChargeBlueEnergyPrim",
    "SwordChargeRedEnergyEnv",
    "SwordChargeRedEnergyPrim",
    "SwordChargeSparksBlue",
    "SwordChargeSparksRed",
    "SwordEnergyBluePrim",
    "SwordEnergyRedPrim",
    "SwordBeamEnergyEnv",
    "SwordBeamEnergyPrim",
    "SwordBeamDamageEnv",
    "BlueBubble",
    "FireArrowEffectEnv",
    "FireArrowEffectPrim",
    "IceArrowEffectEnv",
    "IceArrowEffectPrim",
    "LightArrowEffectEnv",
    "LightArrowEffectPrim",
    "FierceDeityTunic",
    "GoronTunic",
    "ZoraTunic",
    "DekuTunic",
    "HumanTunic",
];

/// "R, G, B" (how the old settings files store colours).
pub(crate) fn parse_rgb(v: &Value) -> Option<Rgb> {
    let parts: Vec<u8> = v.as_str()?.split(',').map(|p| p.trim().parse().ok()).collect::<Option<_>>()?;
    (parts.len() == 3).then(|| Rgb::new(parts[0], parts[1], parts[2]))
}

/// A heart or magic colour choice (`ColorSelectionManager`).
fn pick_pair(random: &mut DotNetRandom, selection: Option<&str>, pairs: &[(Rgb, Rgb)], same_random_color: bool) -> Option<(Rgb, Rgb)> {
    let random_color = |random: &mut DotNetRandom| {
        let mut b = [0u8; 3];
        random.next_bytes(&mut b);
        Rgb::new(b[0], b[1], b[2])
    };
    match selection? {
        "Random Choice" => Some(pairs[if pairs.len() == 1 { 0 } else { random.next_below(pairs.len() as i32) as usize }]),
        "Completely Random" if same_random_color => {
            let c = random_color(random);
            Some((c, c))
        }
        "Completely Random" => {
            let a = random_color(random);
            let b = random_color(random);
            Some((a, b))
        }
        _ => None,
    }
}

type Colors = HashMap<&'static str, Rgb>;

fn adjust_sv(color: Rgb) -> Rgb {
    let up = |x: f32| (x * 3.0).min(1.0);
    translate_hsv(color, |h| Hsv::new(h.h, up(h.s), up(h.v)))
}

impl Builder<'_> {
    pub(crate) fn cosmetic(&self, key: &str) -> &Value {
        self.plan.cosmetic.get(key).unwrap_or(&Value::Null)
    }

    pub(crate) fn write_asm_config(&mut self) -> Result<(), BuildError> {
        let hash = self.hash()?;
        let symbols = self.symbols.clone().ok_or_else(|| BuildError::Data("asm stage hasn't run".into()))?;
        let read_version = |b: &mut Self, addr: u32| b.rom.read_u32(addr + 4);

        // Heart and magic colour choices and UI hue shifts are random,
        // seeded by the hash (`UpdateHudColorOverrides`).
        let mut random = DotNetRandom::new(i32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]));
        let hearts: Vec<(Rgb, Rgb)> = STANDARD_PAIRS.iter().map(|&(a, b)| (b, a)).collect();
        let mut magic: Vec<(Rgb, Rgb)> = vec![(c(0x00, 0xC8, 0x00), c(0x00, 0x00, 0xC8)), (c(0xFF, 0xAE, 0x1A), c(0x6F, 0xD2, 0x51)), (c(0xFF, 0x71, 0xCE), c(0x01, 0xCD, 0xFE))];
        magic.extend(STANDARD_PAIRS);
        let hearts_override = pick_pair(&mut random, self.cosmetic("HeartsSelection").as_str(), &hearts, true);
        let magic_override = pick_pair(&mut random, self.cosmetic("MagicSelection").as_str(), &magic, false);
        let hue_shift = (self.cosmetic("ShiftHueMiscUI").as_bool() == Some(true))
            .then(|| ((random.next_double() * 360.0) as f32, (random.next_double() * 360.0) as f32, (random.next_double() * 360.0) as f32));

        // D-pad.
        let dpad = self.cosmetic("DPad").clone();
        let values: Vec<u8> = dpad["Pad"]["Values"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|v| match v.as_str().unwrap_or("") {
                        "Ocarina" => 0x00,
                        "DekuMask" => 0x32,
                        "GoronMask" => 0x33,
                        "ZoraMask" => 0x34,
                        _ => 0xFF,
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![0x32, 0x34, 0x00, 0x33]);
        let state = u8::from(dpad["State"].as_str() != Some("Disabled"));
        let display = match dpad["Display"].as_str() {
            Some("None") => 0,
            Some("Right") => 2,
            _ => 1,
        };
        if symbols.has("DPAD_STATE") {
            self.rom.write(symbols.get("DPAD_CONFIG")?, &values)?;
            self.rom.write_u8(symbols.get("DPAD_STATE")?, state)?;
        } else {
            let addr = symbols.get("DPAD_CONFIG")?;
            let version = read_version(self, addr)?;
            let mut items = values.clone();
            items.resize(0x10, 0);
            let mut w = version.to_be_bytes().to_vec();
            w.extend(items);
            w.extend([state, display]);
            self.rom.write(addr + 4, &w)?;
        }

        // HUD colours (`HudColorsConfig.FinalizedColors`).
        let hud = self.cosmetic("HudColors").clone();
        let mut colors: Colors = HUD_COLORS.iter().map(|(name, default)| (*name, parse_rgb(&hud[*name]).unwrap_or(*default))).collect();
        let default = |name: &str| HUD_COLORS.iter().find(|(n, _)| *n == name).unwrap().1;
        let a = colors["ButtonA"];
        if a != default("ButtonA") {
            for (k, v) in [
                ("ButtonIconA", a),
                ("MenuAInner1", a),
                ("MenuAInner2", a.brighten(0.4)),
                ("MenuAOuter1", a.darken(0.9)),
                ("MenuAOuter2", a.darken(0.2)),
                ("NoteA1", a),
                ("NoteA2", a.brighten(0.4)),
                ("NoteAShadow1", c(0x0A, 0x0A, 0x0A)),
                ("NoteAShadow2", a.darken(0.2)),
                ("PauseTitleA", a),
                ("Prompt1", a.darken(0.3)),
                ("Prompt2", a),
                ("PromptGlow", a.brighten(0.2)),
                ("ShopCursor1", a.darken(0.4)),
                ("ShopCursor2", a.brighten(0.2)),
            ] {
                colors.insert(k, v);
            }
        }
        if colors["ButtonB"] != default("ButtonB") {
            colors.insert("ButtonIconB", colors["ButtonB"]);
        }
        let cb = colors["ButtonC"];
        if cb != default("ButtonC") {
            for (k, v) in [
                ("ButtonIconC", cb),
                ("MenuCInner1", cb),
                ("MenuCInner2", cb.brighten(0.4)),
                ("MenuCOuter1", c(0, 0, 0)),
                ("MenuCOuter2", cb.darken(0.2)),
                ("NoteC1", cb),
                ("NoteC2", cb.brighten(0.4)),
                ("NoteCShadow1", c(0x0A, 0x0A, 0x0A)),
                ("NoteCShadow2", cb.darken(0.2)),
                ("PauseTitleC", cb),
            ] {
                colors.insert(k, v);
            }
        }
        if let Some((a, b)) = hearts_override {
            colors.insert("Heart", a);
            colors.insert("HeartDD", b);
        }
        if let Some((a, b)) = magic_override {
            colors.insert("Magic", a);
            colors.insert("MagicInf", b);
        }
        if let Some((h1, h2, h3)) = hue_shift {
            for k in ["MenuBorder1", "MenuBorder2"] {
                colors.insert(k, colors[k].rotate_hue(h1));
            }
            colors.insert("MenuSubtitleText", colors["MenuSubtitleText"].rotate_hue(h1).brighten(0.3));
            colors.insert("ScoreLines", colors["ScoreLines"].rotate_hue(h2));
            colors.insert("ScoreNote", colors["ScoreNote"].rotate_hue(h3));
        }
        let addr = symbols.get("HUD_COLOR_CONFIG")?;
        let version = read_version(self, addr)?;
        let count = match version {
            0 => 20,
            1 => 46,
            2 => 47,
            _ => 53,
        };
        let mut w = version.to_be_bytes().to_vec();
        for (name, _) in HUD_COLORS.iter().take(count) {
            let x = colors[name];
            w.extend([x.r, x.g, x.b, 0]);
        }
        self.rom.write(addr + 4, &w)?;

        // World colours: energy effects and tunics.
        self.write_world_colors(&symbols)?;

        // Music: random music gets files for form masks and track names
        // (`MusicConfig.FinalizeSettings`).
        use crate::stages::audio::{NUM_SEQUENCES, SEQUENCE_DATA_SIZE, SEQUENCE_NAME_MAX_SIZE};
        let random_music = self.cosmetic("Music").as_str() == Some("Random");
        self.sequence_mask_file = random_music.then(|| self.rom.append_file(vec![0; NUM_SEQUENCES * SEQUENCE_DATA_SIZE], false));
        self.sequence_names_file =
            (random_music && self.cosmetic("ShowTrackName").as_bool() == Some(true)).then(|| self.rom.append_file(vec![0; NUM_SEQUENCES * SEQUENCE_NAME_MAX_SIZE], false));
        let addr = symbols.get("MUSIC_CONFIG")?;
        let version = read_version(self, addr)?;
        let flag = |k: &str| self.cosmetic(k).as_bool() == Some(true);
        let flags = ((flag("RemoveMinorMusic") as u32) << 31) | ((flag("ShowTrackName") as u32) << 30) | ((flag("DisableFanfares") as u32) << 29);
        let mut w = version.to_be_bytes().to_vec();
        w.extend(self.sequence_mask_file.map_or(0, |f| f as i32).to_be_bytes());
        w.extend(self.sequence_names_file.map_or(0, |f| f as i32).to_be_bytes());
        w.extend(flags.to_be_bytes());
        self.rom.write(addr + 4, &w)?;

        // The hash, shown as icons on the file select screen.
        let mut h = hash.clone();
        h.resize(0x10, 0);
        self.rom.write(symbols.get("MISC_CONFIG")? + 8, &h[..0x10])?;
        self.hash = Some(hash);
        Ok(())
    }

    /// Energy colours per form (`WorldColorsConfig.FinalizeSettings`).
    fn world_colors(&self) -> Colors {
        let mut w: Colors = WORLD_COLORS.iter().copied().collect();
        let energy = self.cosmetic("EnergyColors").clone();
        let colors_of = |form: &str| -> Vec<Rgb> { energy[form].as_array().map(|a| a.iter().filter_map(parse_rgb).collect()).unwrap_or_default() };
        let use_energy = self.cosmetic("UseEnergyColors").clone();
        let forms: Vec<String> = use_energy.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default();
        for form in forms {
            if use_energy[&form].as_bool() != Some(true) {
                continue;
            }
            let colors = colors_of(&form);
            match form.as_str() {
                "Human" => {
                    let (blue, red) = (colors[0], colors[1]);
                    let (blue2, red2) = (adjust_sv(blue), adjust_sv(red));
                    for (k, v) in [
                        ("SwordEnergyBlueEnv1", blue),
                        ("SwordEnergyBlueEnv2", blue2),
                        ("SwordEnergyBluePrim", WHITE),
                        ("SwordEnergyRedEnv1", red),
                        ("SwordEnergyRedEnv2", red2),
                        ("SwordEnergyRedPrim", WHITE),
                        ("SwordChargeBlueEnergyEnv", blue),
                        ("SwordChargeBlueEnergyPrim", WHITE),
                        ("SwordChargeRedEnergyEnv", red),
                        ("SwordChargeRedEnergyPrim", WHITE),
                        ("SwordChargeSparksBlue", blue2),
                        ("SwordChargeSparksRed", red2),
                    ] {
                        w.insert(k, v);
                    }
                }
                "Deku" => {
                    let x = colors[0];
                    w.insert("DekuDustInner", x.brighten(0.2));
                    w.insert("DekuDustOuter", x.darken(0.4));
                    w.insert("DekuSparklesInner", x.brighten(0.7));
                    w.insert("DekuSparklesOuter", x);
                }
                "Goron" => {
                    let (main, extra1, extra2) = (colors[0], colors[1], colors[2]);
                    let mix = |color: Rgb, env: Rgb| {
                        let cur = Hsv::from_rgb(color);
                        let env = Hsv::from_rgb(env);
                        let delta = ((cur.v + env.v) / 2.0) - cur.v;
                        let value = cur.v + (delta * cur.s);
                        (Hsv::new(cur.h, cur.s, value).to_rgb(), Hsv::new(cur.h, cur.s, cur.v).to_rgb())
                    };
                    w.insert("GoronPunchEnergyPrim", translate_hsv(main, |h| Hsv::new(h.h, (h.s / 5.0) * 4.0, h.v)));
                    w.insert("GoronPunchEnergyEnv1", extra1);
                    w.insert("GoronPunchEnergyEnv2", extra1);
                    for (color, env, prim, base) in [
                        (main, "GoronRollInnerEnergyEnv", "GoronRollInnerEnergyPrim", w["GoronRollInnerEnergyEnv"]),
                        (extra1, "GoronRollOuterEnergyEnv1", "GoronRollOuterEnergyPrim1", w["GoronRollOuterEnergyEnv1"]),
                        (extra2, "GoronRollOuterEnergyEnv2", "GoronRollOuterEnergyPrim2", w["GoronRollOuterEnergyEnv2"]),
                    ] {
                        let (e, p) = mix(color, base);
                        w.insert(env, e);
                        w.insert(prim, p);
                    }
                }
                "Zora" => {
                    let x = colors[0];
                    w.insert("ZoraEnergyEnv1", translate_hsv(x, |h| Hsv::new((h.h + 35.0) % 360.0, h.s, h.v * 0.392)));
                    w.insert("ZoraEnergyEnv2", x);
                    w.insert("ZoraEnergyPrim1", x);
                    w.insert("ZoraEnergyPrim2", WHITE);
                }
                "FierceDeity" => {
                    let x = colors[0];
                    w.insert("SwordBeamDamageEnv", adjust_sv(x));
                    w.insert("SwordBeamEnergyEnv", x);
                    w.insert("SwordBeamEnergyPrim", WHITE);
                    w.insert("FierceDeitySparklesInner", x.brighten(0.5));
                    w.insert("FierceDeitySparklesOuter", x);
                }
                _ => {}
            }
        }
        // Tunics.
        for (form, key, default) in [
            ("Human", "HumanTunic", c(0x1E, 0x69, 0x1B)),
            ("FierceDeity", "FierceDeityTunic", c(0xBD, 0xB5, 0xAD)),
            ("Goron", "GoronTunic", c(0x1E, 0x69, 0x1B)),
            ("Zora", "ZoraTunic", c(0x1E, 0x69, 0x1B)),
            ("Deku", "DekuTunic", c(0x1E, 0x69, 0x1B)),
        ] {
            let on = self.cosmetic("UseTunicColors")[form].as_bool() == Some(true);
            w.insert(key, if on { parse_rgb(&self.cosmetic("TunicColors")[form]).unwrap_or(default) } else { default });
        }
        w
    }

    fn write_world_colors(&mut self, symbols: &crate::asm::Symbols) -> Result<(), BuildError> {
        let w = self.world_colors();
        let rgba = |k: &str, a: u8| [w[k].r, w[k].g, w[k].b, a];

        // Energy colours in the player's objects (`WorldColorsConfig.PatchObjects`).
        let human = self.object_data(1)?;
        for (offset, k) in [(0x25850, "SwordEnergyBlueEnv1"), (0x25970, "SwordEnergyBlueEnv2"), (0x25DD0, "SwordEnergyRedEnv1"), (0x25EF0, "SwordEnergyRedEnv2")] {
            self.rom.files[human].data[offset + 0xA4..offset + 0xA8].copy_from_slice(&rgba(k, 0x80));
        }
        const PLAYER: usize = 38;
        let deku = 0x2FEA0;
        for (o, k, a) in [(0, "DekuDustInner", 0xFF), (4, "DekuDustOuter", 0), (8, "DekuSparklesInner", 0), (0xC, "DekuSparklesOuter", 0)] {
            self.rom.files[PLAYER].data[deku + o..deku + o + 4].copy_from_slice(&rgba(k, a));
        }
        let goron = self.object_data(0x14C)?;
        let g = &mut self.rom.files[goron].data;
        g[0x11AB8 + 0x1C..0x11AB8 + 0x20].copy_from_slice(&rgba("GoronPunchEnergyPrim", 0xFF));
        g[0x127CC..0x127D0].copy_from_slice(&rgba("GoronRollInnerEnergyPrim", 0xFF));
        g[0x14660..0x14664].copy_from_slice(&rgba("GoronRollOuterEnergyPrim1", 0xFF));
        g[0x14665..0x14669].copy_from_slice(&rgba("GoronRollOuterEnergyPrim2", 0xFF));
        g[0x1466C..0x14670].copy_from_slice(&rgba("GoronRollOuterEnergyEnv1", 0xFF));
        g[0x14670..0x14674].copy_from_slice(&rgba("GoronRollOuterEnergyEnv2", 0xFF));
        self.rom.write(0xB3C000 + 0x11A320, &rgba("GoronPunchEnergyEnv2", 0))?;
        let zora = self.object_data(0x14D)?;
        let z = &mut self.rom.files[zora].data;
        z[0x11760 + 0x1C..0x11760 + 0x20].copy_from_slice(&rgba("ZoraEnergyPrim1", 0x82));
        z[0x11760 + 0x24..0x11760 + 0x28].copy_from_slice(&rgba("ZoraEnergyEnv1", 0xFF));
        z[0x11760 + 0x19C..0x11760 + 0x1A0].copy_from_slice(&rgba("ZoraEnergyPrim2", 0x82));
        z[0x11760 + 0x1A4..0x11760 + 0x1A8].copy_from_slice(&rgba("ZoraEnergyEnv2", 0xFF));
        let fd = 0x2F964;
        self.rom.files[PLAYER].data[fd..fd + 4].copy_from_slice(&rgba("FierceDeitySparklesInner", 0));
        self.rom.files[PLAYER].data[fd + 4..fd + 8].copy_from_slice(&rgba("FierceDeitySparklesOuter", 0));

        let addr = symbols.get("WORLD_COLOR_CONFIG")?;
        let version = self.rom.read_u32(addr + 4)?;
        let flag = |k: &str| self.cosmetic(k).as_bool() == Some(true);
        let flags = ((flag("RainbowTunic") as u32) << 31) | ((flag("BombTrapsRandomizeTunicColor") as u32) << 30);
        let mut out = version.to_be_bytes().to_vec();
        for k in WORLD_STRUCT {
            out.extend(rgba(k, 0));
        }
        out.extend(flags.to_be_bytes());
        self.rom.write(addr + 4, &out)?;
        Ok(())
    }

    /// The (opened) file of an object (`ObjUtils.GetObjectData`).
    pub(crate) fn object_data(&mut self, object: u32) -> Result<usize, BuildError> {
        const OBJECT_TABLE: u32 = 0xC5_8C80;
        let address = self.rom.read_u32(OBJECT_TABLE + object * 8)?;
        let index = self.rom.files.iter().position(|f| f.addr == address).ok_or_else(|| BuildError::Data(format!("object {object:#X} not found")))?;
        self.rom.open(index)?;
        Ok(index)
    }

    /// The build hash: the old randomizer's when comparing with it, else a
    /// SHA-256 of every file that changed (the same on every OS).
    fn hash(&self) -> Result<Vec<u8>, BuildError> {
        if let Some(h) = &self.plan.hash {
            return Ok(h.clone());
        }
        Ok(crate::hash::content_hash(&self.rom, self.original_files))
    }
}
