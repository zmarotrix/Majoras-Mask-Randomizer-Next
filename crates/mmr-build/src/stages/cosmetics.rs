//! Cosmetic changes (`WriteMiscellaneousChanges`, `WriteTatlColour`,
//! `WriteInstruments`, sound effects, music toggles, and moving the audio
//! bank table).

use crate::stages::hacks::addresses;
use crate::dotnet_random::DotNetRandom;
use crate::message::MessageTable;
use crate::{enums, BuildError, Builder};

/// Where the audio bank table is moved to (unused `fbdemo` code).
pub(crate) const NEW_INSTRUMENT_SET_ADDRESS: u32 = 0xB3C000 + 0xBE300 + 0x10;

/// Tatl's colours per scheme (normal, npc, check, enemy, boss; each a
/// primary/secondary pair): Default, Dark, Hot, Cool.
const TATL_COLOURS: [[u32; 10]; 4] = [
    [0xffffe6ff, 0xdca05000, 0x9696ffff, 0x9696ff00, 0x00ff00ff, 0x00ff0000, 0xffff00ff, 0xc89b0000, 0xffff00ff, 0xc89b0000],
    [0x200020ff, 0x80000000, 0x001080ff, 0x0080ff00, 0x104000ff, 0x80ff0000, 0x800000ff, 0x20002000, 0x800000ff, 0xff800000],
    [0xffc0e0ff, 0xff00ff00, 0xe040ffff, 0xff000000, 0xff80ffff, 0xff00ff00, 0xffe000ff, 0xff000000, 0xff0000ff, 0xff000000],
    [0xc0ffffff, 0x0000ff00, 0xffffffff, 0x00ffff00, 0x00ffffff, 0x00ffff00, 0xc080ffff, 0x0000ff00, 0x8080ffff, 0x0000ff00],
];

impl Builder<'_> {
    pub(crate) fn write_cosmetics(&mut self) -> Result<(), BuildError> {
        self.write_miscellaneous_changes()?;
        self.write_tatl_colour()?;
        self.write_instruments()?;
        self.write_sound_effects()?;
        self.write_low_health_sound()?;
        self.move_audio_bank_table()?;
        if self.cosmetic("Music").as_str() == Some("None") {
            self.write_mute_music()?;
        }
        if self.cosmetic("DisableCombatMusic").as_bool() == Some(true) {
            self.rom.write_u16(0xCA7F00 + 0x16818, 0x1000)?;
        }
        if self.cosmetic("RemoveMinorMusic").as_bool() == Some(true) {
            self.apply_mod("mods", "remove_minor_music")?;
        }
        if self.cosmetic("DisableFanfares").as_bool() == Some(true) {
            self.apply_mod("mods", "remove_fanfares")?;
            // Skulltulas that don't reset with the Song of Time.
            if self.rom.read_u32(0xBDA9DC)? != 0 {
                self.rom.write_u32(0xDFF8B8, 0x0C06_7C32)?; // JAL PlaySfx
                self.rom.write_u32(0xDFF8BC, 0x2404_4824)?; // NA_SE_SY_GET_ITEM
                self.rom.write_u32(0xDFF8D4, 0x0C06_7C32)?;
                self.rom.write_u32(0xDFF8D8, 0x2404_4824)?;
            }
            // Stray fairies that still play the fanfare when all are found.
            if self.rom.read_u32(0xF32F24)? != 0 {
                self.rom.write_u32(0xF32F24, 0x0C06_7C32)?;
                self.rom.write_u32(0xF32F28, 0x2404_4824)?;
            }
        }
        Ok(())
    }

    fn write_miscellaneous_changes(&mut self) -> Result<(), BuildError> {
        if self.cosmetic("EnableHoldZTargeting").as_bool() == Some(true) {
            self.apply_mod("mods", "ztargetinghold")?;
        }
        if self.cosmetic("EnableNightBGM").as_bool() == Some(true) {
            self.reenable_night_bgm()?;
        }
        if self.cosmetic("KeepPictoboxAntialiasing").as_bool() != Some(true) {
            self.apply_mod("mods", "instant_pictobox")?;
        }
        match self.cosmetic("CameraStyle").as_str() {
            Some("Responsive") => {
                self.write_code_u32(0x800D_F44C, 0)?;
                self.write_code_u32(0x800D_F450, 0)?;
            }
            Some("Instant") => self.apply_mod("mods", "improved_camera")?,
            _ => {}
        }
        // Let masks on the D-pad be equipped over.
        let dpad = self.cosmetic("DPad").clone();
        if dpad["State"].as_str() != Some("Disabled") {
            const FILE_KALEIDO_SCOPE: u32 = 0xC90550;
            const MASK_PLAYER_FORM_ITEMS: u32 = 0x1562C;
            let values: Vec<&str> = dpad["Pad"]["Values"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
            for (form_id, mask) in [(1u32, "GoronMask"), (2, "ZoraMask"), (3, "DekuMask")] {
                if values.contains(&mask) {
                    self.rom.write_u8(FILE_KALEIDO_SCOPE + MASK_PLAYER_FORM_ITEMS + form_id, 0xFF)?;
                }
            }
        }
        self.write_crash_debugger_show()?;
        // Dolphin / Wii VC audio thread shutdown workaround.
        self.rom.write_u32(0xB3C000 + 0x0CD384, 0)?;
        Ok(())
    }

    /// Show the crash debugger's information without the secret button
    /// combo, and make it more readable (`WriteCrashDebuggerShow`).
    fn write_crash_debugger_show(&mut self) -> Result<(), BuildError> {
        {
            let boot = &mut self.rom.files[1].data;
            let w32 = |d: &mut Vec<u8>, at: usize, v: u32| d[at..at + 4].copy_from_slice(&v.to_be_bytes());
            w32(boot, 0x3998, 0x240F_0000);
            w32(boot, 0x399C, 0x1000_000A);
            w32(boot, 0x39A0, 0xA1CF_07CF);
            w32(boot, 0x39D0, 0);
            let swap = |d: &mut Vec<u8>, start: usize, end: usize, from: u8, to: u8| {
                for b in &mut d[start..end] {
                    if *b == from {
                        *b = to;
                    }
                }
            };
            swap(boot, 0x185E0, 0x18720, b'H', b' ');
            swap(boot, 0x18760, 0x188D0, b'H', b' ');
            swap(boot, 0x181E0, 0x18230, b'x', b'X');
            swap(boot, 0x18470, 0x18B04, b'x', b'X');
            w32(boot, 0x31D8, 0);
            w32(boot, 0x2DA4, 0x00A0_B025);
            w32(boot, 0x3A38, 0);
            w32(boot, 0x3A4C, 0);
            w32(boot, 0x2EBC, 0x1000_0022);
        }
        let code = &mut self.rom.files[31].data;
        let put = |d: &mut Vec<u8>, at: usize, bytes: &[u8]| d[at..at + bytes.len()].copy_from_slice(bytes);
        put(code, 0x137104, b"A.Id RAMStart -   RAMEnd Num.");
        put(code, 0x137124, b"%4d %08X - %08X %4d ");
        code[0xE088] = 0x96;
        code[0xE08B] = 0x1C;
        code[0xE0B0] = 0x96;
        code[0xE0B3] = 0x1C;
        put(code, 0x136F18, b"A.Grp  RAM       A.Id Params ");
        put(code, 0x136F38, b"%5d  %08X  %4X %04X ");
        put(code, 0x19F00, &0x0000_2725u32.to_be_bytes());
        put(code, 0xE034, &0x0000_2725u32.to_be_bytes());
        Ok(())
    }

    fn write_tatl_colour(&mut self) -> Result<(), BuildError> {
        let scheme = self.cosmetic("TatlColorSchema").as_str().unwrap_or("Default").to_string();
        if scheme == "Rainbow" {
            return self.apply_mod("mods", "rainbow_tatl");
        }
        let colours = match scheme.as_str() {
            "Dark" => TATL_COLOURS[1],
            "Hot" => TATL_COLOURS[2],
            "Cool" => TATL_COLOURS[3],
            "Random" => {
                // Alpha (the low byte) on for primaries, off for secondaries.
                let mut random = self.cosmetic_random();
                let mut colours = [0u32; 10];
                for (i, colour) in colours.iter_mut().enumerate() {
                    let mut c = [0u8; 4];
                    random.next_bytes(&mut c);
                    c[0] = if i % 2 == 0 { 0xFF } else { 0 };
                    *colour = u32::from_le_bytes(c);
                }
                colours
            }
            _ => TATL_COLOURS[0],
        };
        let locations = addresses(crate::resources::addresses("tatl_colour").unwrap());
        // Like the old code, one buffer is reused for every location.
        let mut c = [0u8; 8];
        for (i, addrs) in locations.iter().enumerate() {
            c[0..4].copy_from_slice(&colours[i << 1].to_be_bytes());
            c[4..8].copy_from_slice(&colours[(i << 1) + 1].to_be_bytes());
            self.write_rom_addr(addrs, &mut c)?;
        }
        Ok(())
    }

    /// The random numbers cosmetics use: .NET's `Random`, seeded by the hash.
    fn cosmetic_random(&self) -> DotNetRandom {
        let h = self.hash.as_deref().unwrap_or(&[0; 4]);
        DotNetRandom::new(i32::from_le_bytes([h[0], h[1], h[2], h[3]]))
    }

    /// Play the day music at night in the overworld (`SceneUtils.ReenableNightBGM`).
    fn reenable_night_bgm(&mut self) -> Result<(), BuildError> {
        const SCENES: [&str; 27] = [
            "TerminaField",
            "RoadToSouthernSwamp",
            "SouthernSwamp",
            "SouthernSwampClear",
            "PathToMountainVillage",
            "MountainVillage",
            "MountainVillageSpring",
            "TwinIslands",
            "TwinIslandsSpring",
            "GoronRacetrack",
            "GoronVillage",
            "GoronVillageSpring",
            "PathToSnowhead",
            "Snowhead",
            "MilkRoad",
            "GreatBayCoast",
            "PinnacleRock",
            "ZoraCape",
            "WaterfallRapids",
            "RoadToIkana",
            "IkanaCanyon",
            "EastClockTown",
            "WestClockTown",
            "NorthClockTown",
            "SouthClockTown",
            "LaundryPool",
            "Woodfall",
        ];
        let scenes = enums::get("Scene");
        for name in SCENES {
            let id = scenes.attr(name, "SceneInternalId").and_then(|a| a.get("InternalId").as_u64()).ok_or_else(|| BuildError::Data(format!("scene {name}")))?;
            let file = self.scenes.iter().find(|s| s.number as u64 == id).ok_or_else(|| BuildError::Data(format!("scene {name} not in the table")))?.file;
            let data = self.rom.file(file)?;
            // The sound settings header command (0x15): its night music byte.
            if let Some(at) = (0..0x10 * 70).step_by(8).find(|&b| data[b] == 0x15) {
                data[at + 6] = 0x13;
            }
        }
        // Kamaro and Sakon stop the night music; skip the calls that do it.
        self.rom.file(593)?[0x618..0x61C].fill(0);
        self.rom.file(526)?[0x3A0C..0x3A10].fill(0);
        Ok(())
    }

    /// Each form's instrument (`WriteInstruments`).
    fn write_instruments(&mut self) -> Result<(), BuildError> {
        const CODE: u32 = 0xB3C000;
        const MILK_BAR: u32 = 0x46AF0 + 0x3AC90;
        const PLAYBACK: u32 = CODE + 0x12A8DC;
        const FREE_PLAY: u32 = CODE + 0x12A8E4;
        const FREE_PLAY_ARRAY: u32 = 0x51CBE;
        let instruments = enums::get("Instrument");
        let id = |name: &str| instruments.attr(name, "Id").and_then(|a| a.get("Id").as_u64()).map(|v| v as u8);
        let chosen = self.cosmetic("Instruments").clone();
        // (form, index, milk bar offset, default); random ones go last.
        let mut forms = vec![("Human", 0u32, 0x85, "Ocarina"), ("Deku", 3, 0xE6, "DekuPipes"), ("Goron", 1, 0x16A, "Drums"), ("Zora", 2, 0x158, "Guitar")];
        forms.sort_by_key(|f| chosen[f.0].as_str() == Some("Random"));
        let mut random = self.cosmetic_random();
        let mut milk_bar = vec![(CODE + 0x13BB0A, 0u8), (MILK_BAR + 0x178, id("FemaleVoice").unwrap())];
        let mut changed = false;
        let mut used: Vec<String> = Vec::new();
        for (form, index, milk_offset, default) in forms {
            let mut instrument = chosen[form].as_str().unwrap_or(default).to_string();
            if instrument == default {
                milk_bar.push((MILK_BAR + milk_offset, id(&instrument).unwrap()));
                used.push(instrument);
                continue;
            }
            changed = true;
            if instrument == "Random" {
                let pool: Vec<&str> = instruments.values.iter().map(|v| v.name.as_str()).filter(|n| !used.iter().any(|u| u == n)).skip(1).collect();
                instrument = pool[if pool.len() == 1 { 0 } else { random.next_below(pool.len() as i32) as usize }].to_string();
            }
            let value = id(&instrument).ok_or_else(|| BuildError::Data(format!("instrument {instrument}")))?;
            milk_bar.push((MILK_BAR + milk_offset, value));
            used.push(instrument);
            let free_play = self.rom.read_u8(FREE_PLAY + index)?.wrapping_sub(1);
            self.rom.write_u8(FREE_PLAY_ARRAY + free_play as u32, value)?;
            self.rom.write_u8(PLAYBACK + index, value)?;
        }
        if changed {
            for (addr, value) in milk_bar {
                self.rom.write_u8(addr, value)?;
            }
        }
        Ok(())
    }

    /// Shuffle sound effects among those with matching tags (`WriteSoundEffects`).
    fn write_sound_effects(&mut self) -> Result<(), BuildError> {
        if self.cosmetic("RandomizeSounds").as_bool() != Some(true) {
            return Ok(());
        }
        let mut random = self.cosmetic_random();
        let all = sound_effects();
        // Insertion-ordered, like a .NET dictionary that's only added to.
        let mut shuffled: Vec<(u16, u16)> = Vec::new();
        for sound in &all {
            let pool = sounds_with_tags(&all, &replacable_by_tags(sound));
            if pool.is_empty() {
                continue;
            }
            let pick = pool[if pool.len() == 1 { 0 } else { random.next_below(pool.len() as i32) as usize }];
            match shuffled.iter_mut().find(|(k, _)| *k == sound.value) {
                Some(entry) => entry.1 = pick,
                None => shuffled.push((sound.value, pick)),
            }
        }
        let low_health = sound_effect("LowHealthBeep").value;
        shuffled.retain(|(k, _)| *k != low_health);
        for &(old, new) in &shuffled {
            let sound = all.iter().find(|s| s.value == old).unwrap();
            self.replace_sound(sound, new)?;
        }
        // Sound effects in text (0x1E xx xx), the first one in each message.
        let mut table = MessageTable::read_default(&mut self.rom)?;
        let entries: Vec<_> = table.iter().cloned().collect();
        for mut entry in entries {
            let m = &entry.message;
            let control = |i: usize| (0x1B..=0x1F).contains(&m[i]);
            let found = (0..m.len().saturating_sub(2)).find(|&i| m[i] == 0x1E && !(i >= 1 && control(i - 1)) && !(i >= 2 && control(i - 2)));
            let Some(i) = found else { continue };
            let old = ((m[i + 1] & 0xFF) << 8) | (m[i + 2] & 0xFF);
            let flag = old & 0x0800;
            if let Some(&(_, new)) = shuffled.iter().find(|(k, _)| *k == old & !0x0800) {
                if new != 0 {
                    let id = new | flag;
                    entry.message[i + 1] = id >> 8;
                    entry.message[i + 2] = id & 0xFF;
                    table.update(entry);
                }
            }
        }
        table.write_default(&mut self.rom, false)
    }

    /// `WriteLowHealthSound`.
    fn write_low_health_sound(&mut self) -> Result<(), BuildError> {
        let choice = self.cosmetic("LowHealthSFX").as_str().unwrap_or("Default").to_string();
        let beep = sound_effect("LowHealthBeep");
        match choice.as_str() {
            "Default" => Ok(()),
            "Disabled" => {
                for addr in beep.addresses.clone() {
                    self.rom.write_u16(addr, 0)?;
                }
                Ok(())
            }
            "Random" => {
                let all = sound_effects();
                let pool = sounds_with_tags(&all, &replacable_by_tags(&beep));
                if pool.is_empty() {
                    return Ok(());
                }
                let mut random = self.cosmetic_random();
                let pick = pool[if pool.len() == 1 { 0 } else { random.next_below(pool.len() as i32) as usize }];
                self.replace_sound(&beep, pick)
            }
            name => {
                let target = enums::get("LowHealthSFX").attr(name, "Value").and_then(|a| a.get("Value").as_str()).ok_or_else(|| BuildError::Data(format!("low health sound {name}")))?;
                self.replace_sound(&beep, sound_effect(target).value)
            }
        }
    }

    /// `SoundEffectExtensions.TryReplaceWith`.
    fn replace_sound(&mut self, sound: &Sound, new: u16) -> Result<(), BuildError> {
        if sound.addresses.is_empty() {
            return Ok(());
        }
        let value = match sound.effect_flags {
            Some(flags) => (flags & 0x0E00) | (new & 0xF1FF),
            None => new.wrapping_add(0x0800),
        };
        for &addr in &sound.addresses {
            self.rom.write_u16(addr, value)?;
        }
        Ok(())
    }

    /// Silence every sequence but the milk bar band's (`WriteMuteMusic`).
    fn write_mute_music(&mut self) -> Result<(), BuildError> {
        const SEQ_TABLE: u32 = 0xC77B80;
        const AUDIOSEQ_FILE: usize = 4;
        let code = self.rom.index_of(SEQ_TABLE).ok_or_else(|| BuildError::Data("sequence table".into()))?;
        self.rom.open(code)?;
        self.rom.open(AUDIOSEQ_FILE)?;
        let base = (SEQ_TABLE - self.rom.files[code].addr) as usize;
        for seq in (2..128).filter(|&s| s != 0x54) {
            let at = base + seq * 16;
            let d = &self.rom.files[code].data;
            let location = u32::from_be_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]]) as usize;
            let audioseq = &mut self.rom.files[AUDIOSEQ_FILE].data;
            for b in 3..128 {
                // The master volume command.
                if audioseq[location + b] == 0xDB {
                    audioseq[location + b + 1] = 0;
                }
            }
        }
        Ok(())
    }
}

/// A sound effect and what the old code knew about it (`SoundEffect`).
#[derive(Clone)]
struct Sound {
    value: u16,
    tags: Vec<String>,
    replacable_by: Option<Vec<String>>,
    addresses: Vec<u32>,
    effect_flags: Option<u16>,
}

const SOUND_TAGS: [&str; 5] = ["Short", "Long", "LowHpBeep", "SystemSound", "Looping"];

fn strings(v: &serde_json::Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn to_sound(v: &enums::EnumValue) -> Sound {
    let attr = |kind: &str| v.attributes.iter().find(|a| a.kind == kind);
    Sound {
        value: v.value as u16,
        tags: attr("Tags").map(|a| strings(a.get("Tags"))).unwrap_or_default(),
        replacable_by: attr("ReplacableByTags").map(|a| strings(a.get("Tags"))),
        addresses: attr("Replacable").and_then(|a| a.get("Addresses").as_array().map(|x| x.iter().filter_map(|n| n.as_u64()).map(|n| n as u32).collect())).unwrap_or_default(),
        effect_flags: attr("Effect").and_then(|a| a.get("Flags").as_u64()).map(|f| f as u16),
    }
}

/// Every sound effect, in value order (`Enum.GetValues`).
fn sound_effects() -> Vec<Sound> {
    let mut all: Vec<Sound> = enums::get("SoundEffect").values.iter().map(to_sound).collect();
    all.sort_by_key(|s| s.value);
    all
}

fn sound_effect(name: &str) -> Sound {
    to_sound(enums::get("SoundEffect").get(name).unwrap_or_else(|| panic!("sound effect {name}")))
}

/// The tags a sound's replacement may have (`ReplacableByTags`).
fn replacable_by_tags(sound: &Sound) -> Vec<String> {
    match &sound.replacable_by {
        Some(t) => t.clone(),
        None if !sound.tags.is_empty() => sound.tags.clone(),
        None => SOUND_TAGS.iter().map(|s| s.to_string()).collect(),
    }
}

fn sounds_with_tags(all: &[Sound], tags: &[String]) -> Vec<u16> {
    all.iter().filter(|s| tags.iter().any(|t| s.tags.contains(t))).map(|s| s.value).collect()
}

impl Builder<'_> {
    /// Move the audio bank table somewhere with room for more banks
    /// (`SequenceUtils.MoveAudioBankTable`).
    fn move_audio_bank_table(&mut self) -> Result<(), BuildError> {
        let table = self.rom.bytes_mut(0xB3C000 + 0x13B6C0, 0x820)?.to_vec();
        self.rom.write(0xB3C000 + 0xBE300, &table)?;
        self.rom.write_u16(0xB3C000 + 0xBE300, 0x0080)?;
        self.write_code_u32(0x8019_0E18, 0x3C0A_8016)?;
        self.write_code_u32(0x8019_0E28, 0x254A_3DC0)?;
        self.rom.write(0xB3C000 + 0x13B7D0, &[0u8; 0x190])?;
        self.apply_mod("mods", "instrumentset_patch")?;
        self.apply_mod("mods", "moveaudiostatebytes")?;
        // Dummy banks (copies of Kamaro's dance bank metadata).
        let mut at = NEW_INSTRUMENT_SET_ADDRESS + 0x280;
        for _ in 0..=0x58 {
            self.rom.write(at, &0x0002_1880_0000_00D0u64.to_be_bytes())?;
            self.rom.write(at + 8, &0x0201_01FF_0100_0000u64.to_be_bytes())?;
            at += 0x10;
        }
        Ok(())
    }
}
