//! Gameplay gimmicks and misc hacks (`WriteGimmicks`, `WriteMiscHacks`, `WriteMoonChildDenialHack`).

use crate::enums;
use crate::message::{message, Header, TextBox};
use crate::{BuildError, Builder};
use serde_json::Value;

impl Builder<'_> {
    /// Pick one of several patches by a setting's number (1-based; 0 = none).
    fn indexed_mod(&mut self, key: &str, patches: &[(&str, &str)]) -> Result<(), BuildError> {
        let n = self.plan.enum_value(key);
        if n > 0 {
            let (group, name) = patches[(n - 1) as usize];
            self.apply_mod(group, name)?;
        }
        Ok(())
    }

    pub(crate) fn write_gimmicks(&mut self) -> Result<(), BuildError> {
        let p = self.plan;
        self.indexed_mod("DamageMode", &[("mods", "dm_1"), ("mods", "dm_2"), ("mods", "dm_8x"), ("mods", "dm_3"), ("asm", "DamageModeDoom")])?;
        self.indexed_mod("DeathMode", &[("asm", "DeathMoonCrash"), ("mods", "death_reduces_max_hearts")])?;
        self.indexed_mod("DamageEffect", &[("mods", "de_1"), ("mods", "de_2"), ("mods", "de_3"), ("mods", "de_4"), ("mods", "de_5")])?;
        self.indexed_mod("MovementMode", &[("mods", "movement_1"), ("mods", "movement_2"), ("mods", "movement_3"), ("mods", "movement_4")])?;
        self.indexed_mod("FloorType", &[("mods", "floor_1"), ("mods", "floor_2"), ("mods", "floor_3"), ("mods", "floor_4")])?;
        if p.choice("ClockSpeed") != "Default" {
            self.write_clock_speed()?;
        }
        if p.flag("HideClock") {
            // Always take the "time hasn't changed" branch for the clock UI.
            self.rom.write_u8(0xB3C000 + 0x73B7C, 0x10)?;
        }
        if p.choice("BlastMaskCooldown") != "Default" {
            let value: u16 = match p.choice("BlastMaskCooldown") {
                "Instant" => 0x1,
                "VeryShort" => 0x20,
                "Short" => 0x80,
                "Long" => 0x200,
                "VeryLong" => 0x400,
                _ => 0x136,
            };
            self.rom.write_u16(0x00CA7F00 + 0x002766, value)?;
        }
        if p.flag("EnableSunsSong") {
            self.messages.update(message(0x1B7D, Some(Header::new(TextBox::OcarinaStaff)), |it| {
                it.text("You played the ").yellow("Sun's Song").text("!").end_final_text_box();
            }));
            self.apply_mod("mods", "enable_sunssong")?;
        }
        if p.flag("AllowFierceDeityAnywhere") {
            self.apply_mod("mods", "fierce_deity_anywhere")?;
        }
        if p.flag("AllowFierceDeityAnywhere") || p.flag("SaferGlitches") {
            self.apply_mod("mods", "safer_glitches_fierce_deity")?;
        }
        if p.flag("GiantMaskAnywhere") {
            self.apply_mod("mods", "giant_mask_anywhere")?;
        }
        if p.flag("ByoAmmo") {
            self.apply_mod("mods", "byo_ammo")?;
        }
        if p.flag("HookshotAnySurface") {
            self.apply_mod("mods", "hookshot_any_surface")?;
        }
        if p.flag("ClimbMostSurfaces") {
            self.apply_mod("mods", "climb_most_surfaces")?;
        }
        if !p.flag("VanillaMoonTrialAccess") {
            self.apply_mod("mods", "fix_moon_trial_access")?;
        }
        Ok(())
    }

    fn write_clock_speed(&mut self) -> Result<(), BuildError> {
        let (speed, inverted): (u8, i16) = match self.plan.choice("ClockSpeed") {
            "VerySlow" => (1, 0),
            "Slow" => (2, -1),
            "Fast" => (6, -4),
            "VeryFast" => (9, -6),
            "SuperFast" => (18, -12),
            _ => (3, -2),
        };
        self.apply_mod("mods", "fix_clock_speed")?;
        self.rom.write_u8(0xB3C000 + 0x8A674 + 0x1B, speed)?;
        for offset in [0xB1B8E, 0x7405E] {
            self.rom.write_u16(0xB3C000 + offset, inverted as u16)?;
        }
        Ok(())
    }

    pub(crate) fn write_misc_hacks(&mut self) -> Result<(), BuildError> {
        let p = self.plan;
        let mut hacks: Vec<Vec<u8>> = Vec::new();
        let named = |name: &str| crate::resources::mods(name).unwrap().to_vec();
        let add_value_patches = |hacks: &mut Vec<Vec<u8>>, enum_name: &str, value: &str| {
            for a in enums::get(enum_name).attrs(value, "HackContent") {
                if let Some(name) = a.args.first().and_then(Value::as_str) {
                    hacks.push(named(name));
                }
            }
        };
        if p.has("SmallKeyMode", "DoorsOpen") {
            add_value_patches(&mut hacks, "SmallKeyMode", "DoorsOpen");
        }
        if p.has("BossKeyMode", "DoorsOpen") {
            add_value_patches(&mut hacks, "BossKeyMode", "DoorsOpen");
        }
        if p.has("BossKeyMode", "KeepThroughTime") {
            add_value_patches(&mut hacks, "BossKeyMode", "KeepThroughTime");
        }
        let mut required_fairies: u16 = 15;
        if p.has("StrayFairyMode", "ChestsOnly") {
            required_fairies = 0;
            add_value_patches(&mut hacks, "StrayFairyMode", "ChestsOnly");
        }
        required_fairies += 0xA; // needed for the value to be correct
        self.rom.write_u16(0x00EA3366, required_fairies)?;

        if p.flag("LenientGoronSpikes") {
            hacks.push(named("lenient_goron_spikes"));
        }
        if p.flag("TargetHealthBar") {
            hacks.push(named("enemy_max_health"));
        }
        if p.choice("LogicMode") != "Vanilla" {
            let starting = |i: &str| p.custom_starting_item_list.iter().any(|x| x == i);
            let randomized = |i: &str| p.custom_item_list.iter().any(|x| x == i);
            if !starting("ItemOcarina") || !starting("SongTime") || randomized("ItemOcarina") || randomized("SongTime") {
                hacks.push(named("fix_ocarina_checks"));
                hacks.push(named("fix_song_of_time"));
            }
        }
        if p.choice("GaroHintStyle") != "Default" {
            hacks.push(named("garo_hints"));
        }
        if p.choice("ChestGameMinimap") != "Off" {
            self.rom.write_u16(0x02131000 + 0x1D8, 0x0018)?;
            self.rom.write(0x02131000 + 0x1C8, &[0x00, 0x01, 0xFD, 0x00, 0x00, 0x00, 0x02, 0x18, 0x00, 0x00])?;
            self.rom.write_u8(0x00B3C000 + 0x0011C270 + 0x71, 0x80)?;
            self.rom.write(0x01128000 + 0x42C8, crate::resources::mods("chestgame_minimap").unwrap())?;
        }
        if p.flag("SaferGlitches") {
            for name in [
                "safer_glitches_sodt",
                "safer_glitches_tatl_text_zero_fourth_day",
                "safer_glitches_fix_0thday_erase",
                "safer_glitches_fix_goron_bow",
                "safer_glitches_index_warp",
                "safer_glitches_fix_4thday_mayor",
                "safer_glitches_fix_4thday_gossip",
                "safer_glitches_fix_4thday_deku_playground",
                "safer_glitches_fix_0thday_4thday_town_shooting",
            ] {
                hacks.push(named(name));
            }
        }
        if p.flag("BombchuDrops") {
            hacks.push(named("add_bombchu_drops"));
        }
        let remains = p.number("RequiredBossRemains");
        if remains < 4 {
            let mut hack = named("update_remains_required");
            hack[0x33] = remains as u8;
            hacks.push(hack);
        }
        let mut more_hacks: Vec<(u32, Vec<u8>)> = Vec::new();
        let eggs = p.number("RequiredZoraEggs");
        if eggs < 7 {
            let baseline = (7 - eggs) as u8;
            let correction = 1u16.wrapping_sub(baseline as u16);
            hacks.push(crate::resources::asm("ZoraEgg").unwrap().to_vec());
            let symbols = crate::asm::Symbols::from_json(crate::resources::asm("ZoraEgg_symbols").unwrap())?;
            more_hacks.push((symbols.get("ZORA_EGG_BASELINE_ADD")? + 3, vec![baseline]));
            more_hacks.push((symbols.get("ZORA_EGG_BASELINE_SUBTRACT")? + 2, correction.to_be_bytes().to_vec()));
        }
        for (flag, name) in [
            ("TakeDamageWhileShielding", "take_damage_while_shielding"),
            ("TakeDamageFromVoid", "take_damage_from_void"),
            ("TakeDamageFromDog", "take_damage_from_dog"),
            ("TakeDamageFromGorons", "take_damage_from_goron"),
        ] {
            if p.flag(flag) {
                hacks.push(named(name));
            }
        }
        if p.flag("TakeDamageGettingCaught") {
            hacks.push(named("take_damage_from_caught"));
            if p.choice("LogicMode") != "Vanilla" {
                hacks.push(named("sonata_check_gentle_throw"));
            }
        }
        if p.flag("TakeDamageFromGibdosFaster") {
            hacks.push(named("take_damage_from_gibdo_immediately"));
        }
        if p.flag("TakeDamageFromDexihands") {
            hacks.push(named("take_damage_from_dexihands"));
        }
        for hack in hacks {
            self.apply_hack(&hack)?;
        }
        for (address, data) in more_hacks {
            self.rom.write(address, &data)?;
        }
        Ok(())
    }

    /// Make room for an extra object in the Moon's first room
    /// (`WriteMoonChildDenialHack`).
    pub(crate) fn write_moon_child_denial_hack(&mut self) -> Result<(), BuildError> {
        const MOON_ROOM: usize = 1501;
        let data = self.rom.file(MOON_ROOM)?;
        data.drain(0x194..0x198); // less end padding after the actor list
        data.splice(0x44..0x44, [0x01, 0xBE, 0x00, 0x00]); // an extra object
        data[0x29] += 1; // object count
        data[0x37] += 4; // actor list address
        for scene in &mut self.scenes {
            for setup in &mut scene.setups {
                for room in &mut setup.rooms {
                    if room.file == MOON_ROOM {
                        room.actor_list_address = room.actor_list_address.map(|a| a + 4);
                    }
                }
            }
        }
        self.apply_mod("mods", "fix_object_stk2_zbuffer")
    }
}
