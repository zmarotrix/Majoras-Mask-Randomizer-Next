//! Startup text, traps, and the in-game payload (`WriteStartupStrings`,
//! `WriteTraps`, `WriteAsmPatch`, `WriteNutsAndSticks`).

use crate::asm::{self, BitPacker, Symbols};
use crate::message::MessageTable;
use crate::{BuildError, Builder};
use mmr_rom::RomFile;

impl Builder<'_> {
    pub(crate) fn write_startup(&mut self) -> Result<(), BuildError> {
        if self.plan.choice("LogicMode") == "Vanilla" {
            return Ok(());
        }
        self.apply_mod("mods", "logo_text")?;
        let version = self.plan.version.clone().unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION")));
        self.rom.write(0xC44E30, format!("MM Rando {version}\0").as_bytes())?;
        self.rom.write(0xC44E70, b"\0")?;
        Ok(())
    }

    /// Traps: their disguises' graphics, and their messages (`WriteTraps`).
    pub(crate) fn write_traps(&mut self) -> Result<(), BuildError> {
        if self.plan.choice("LogicMode") == "Vanilla" || self.plan.choice("TrapAmount") == "None" {
            return Ok(());
        }
        let traps = self.plan.traps.clone().unwrap_or_default();
        for trap in &traps {
            let Some(location) = trap.location.as_deref() else { continue };
            let visible = crate::items::is_model_visible(self.plan, location)
                && (trap.item != "Rupoor" || crate::items::is_shop_model_visible(location))
                && trap.item != "Nothing";
            if !visible {
                continue;
            }
            let gi = crate::items::get_item_index(location).ok_or_else(|| BuildError::Data(format!("{location} has no get-item index")))?;
            let mimic = trap.mimic.as_ref().map(|m| m.item.as_str()).unwrap_or(&trap.item);
            let entry = match crate::items::get_item_index(mimic) {
                Some(i) => self.get_items[&i],
                None => crate::items::exclusive_entry(mimic).ok_or_else(|| BuildError::Data(format!("{mimic} has no model")))?,
            };
            if self.graphic_overrides.iter().any(|(g, _)| *g == gi) {
                return Err(BuildError::Data(format!("two traps at get-item index {gi:#X}")));
            }
            self.graphic_overrides.push((gi, crate::extended::ItemGraphic { object: entry.object, graphic: entry.index }));
        }
        for item in ["IceTrap", "BombTrap", "Rupoor", "Nothing"] {
            let entry = crate::items::exclusive_entry(item).unwrap();
            let (_, text) = crate::items::exclusive_message(item).unwrap();
            self.extra_messages.push(crate::message::MessageEntry::new(entry.message, crate::message::units(text)));
        }
        Ok(())
    }

    pub(crate) fn write_asm(&mut self) -> Result<(), BuildError> {
        let symbols = Symbols::from_json(crate::resources::asm("symbols").unwrap())?;
        let patch = asm::read_patch(crate::resources::asm("rom_patch").unwrap())?;
        let misc = self.misc_config();

        // Patch existing files, then add the payload, the pause-menu payload
        // and the symbols as files of their own (`Patcher.Apply`).
        let payload_start = symbols.get("PAYLOAD_START")?;
        for e in &patch {
            if e.address < payload_start {
                self.rom.write(e.address, &e.data)?;
            }
        }
        for (start, end) in [("PAYLOAD_START", "PAYLOAD_END"), ("KALEIDOPAYLOAD_START", "KALEIDOPAYLOAD_END")] {
            let (start, end) = (symbols.get(start)?, symbols.get(end)?);
            let data = asm::file_data(&patch, start, end - start);
            self.insert_static(start, data);
        }
        self.insert_static(asm::SYMBOLS_FILE_START, symbols.to_bytes());

        // Configuration compiled into the patch (`Symbols.ApplyConfiguration`).
        let icon = {
            let data = self.rom.file(11)?;
            crate::fairy_icon::clock_town_fairy_icon(&data[0x1B80 + 0xC00..0x1B80 + 0x1800])
        };
        self.rom.write(symbols.get("TOWN_FAIRY_BYTES")?, &icon)?;
        let addr = symbols.get("MISC_CONFIG")?;
        let version = self.rom.read_u32(addr + 4)?;
        self.rom.write(addr + 4, &misc.to_bytes(version))?;
        let addr = symbols.get("MMR_CONFIG")?;
        let version = self.rom.read_u32(addr + 4)?;
        let bytes = self.mmr_config.to_bytes(version).map_err(BuildError::Data)?;
        self.rom.write(addr + 4, &bytes)?;

        let ext_count = self.rom.read_u32(symbols.get("EXT_MSG_TABLE_COUNT")?)?;
        let mimic_count = self.rom.read_u32(symbols.get("ITEM_OVERRIDE_COUNT")?)?;

        // Extended objects, and their addresses in the payload's table.
        if let Some(x) = self.extended.take() {
            let index = self.rom.append_file(x.bundle(), false);
            let base = self.rom.files[index].addr;
            let table = symbols.get("EXT_OBJECTS")?;
            for (i, (start, end)) in x.offsets.iter().enumerate() {
                let at = table + (i as u32 + 1) * 8;
                self.rom.write_u32(at, base + start)?;
                self.rom.write_u32(at + 4, base + end)?;
            }
            self.extended = Some(x);
        }

        // Extra messages go in the extended message table.
        let mut table = MessageTable::new(ext_count as usize);
        for m in std::mem::take(&mut self.extra_messages) {
            table.add(m)?;
        }
        if self.plan.flag("QuickTextEnabled") {
            table.apply_quick_text();
        }
        // A dummy last entry, so the one before it can work out its length.
        table.add(crate::message::MessageEntry::new(0xFFFE, vec![0xBF]))?;
        let (entries, data) = table.build_data()?;
        self.rom.write(symbols.get("EXT_MSG_TABLE")?, &entries)?;
        let index = self.rom.append_file(data, false);
        self.rom.write_u32(symbols.get("EXT_MSG_DATA_FILE")?, index as u32)?;

        // Item graphic overrides (trap disguises).
        let mut mimic = vec![0u8; mimic_count as usize * 4];
        for (gi, g) in &self.graphic_overrides {
            let at = *gi as usize * 4;
            mimic[at..at + 2].copy_from_slice(&(g.graphic as u16).to_be_bytes());
            mimic[at + 2..at + 4].copy_from_slice(&g.object.to_be_bytes());
        }
        self.rom.write(symbols.get("ITEM_OVERRIDE_ENTRIES")?, &mimic)?;

        self.write_nuts_and_sticks()?;
        self.symbols = Some(symbols);
        Ok(())
    }

    /// Add a file at a fixed address, after all others.
    fn insert_static(&mut self, addr: u32, data: Vec<u8>) {
        self.rom.insert_file(RomFile { addr, end: addr + data.len() as u32, is_static: true, data, cmp_addr: Some(0), ..Default::default() });
    }

    /// Deku nuts and sticks in drop tables (`WriteNutsAndSticks`).
    fn write_nuts_and_sticks(&mut self) -> Result<(), BuildError> {
        let bushes = self.plan.enum_value("NutandStickDrops") as u32;
        if bushes == 0 {
            return Ok(());
        }
        const NUT: u8 = 0x0C;
        const STICK: u8 = 0x0D;
        let nuts = if bushes == 1 { 1 } else { bushes as u8 };
        let sticks = (bushes as i32 - 2).max(1) as u8;
        let table = self.rom.index_of(0xC444B8).unwrap();
        self.rom.open(table)?;
        let base = self.rom.files[table].addr;
        let mut add = |slot: u32, drop: u8, amount: u8| {
            let o = (slot - base) as usize;
            self.rom.files[table].data[o] = drop;
            self.rom.files[table].data[o + 0x110] = amount;
        };
        // Termina Field grass.
        add(0xC444B7, NUT, nuts);
        add(0xC444BF, STICK, sticks);
        if bushes >= 2 {
            add(0xC444B8, NUT, nuts);
            add(0xC444C0, STICK, sticks);
        }
        if bushes >= 3 {
            add(0xC444BC, NUT, nuts);
            add(0xC444C1, STICK, sticks);
            add(0xC444CB, NUT, nuts); // Stalchildren and the south swamp
            add(0xC444CC, STICK, sticks);
            add(0xC44574, NUT, nuts); // Lens of Truth grass
            add(0xC44575, STICK, sticks);
        }
        if bushes >= 4 {
            add(0xC444F8, NUT, nuts); // graveyard rocks
            add(0xC444F9, STICK, sticks);
            add(0xC444D6, NUT, nuts); // snow trees and rocks
            add(0xC444D7, STICK, sticks);
            add(0xC445BA, NUT, nuts); // field mice
            add(0xC445BB, STICK, sticks);
        }
        Ok(())
    }

    /// The payload's misc config (`MiscConfig` after `FinalizeSettings`).
    fn misc_config(&self) -> MiscConfig {
        let p = self.plan;
        let general = |f: &str| self.cutscene_general(f);
        let victory = |f: &str| p.has("VictoryMode", f);
        let remains = if victory("FourBossRemains") {
            4
        } else if victory("ThreeBossRemains") {
            3
        } else if victory("TwoBossRemains") {
            2
        } else if victory("OneBossRemains") {
            1
        } else {
            0
        };
        let ocean_tokens = crate::items::range_from("CollectibleOceanSpiderToken1", 30);
        MiscConfig {
            flags: {
                let mut b = BitPacker::default();
                b.int(if p.flag("CritWiggleDisable") { 2 } else { 0 }, 2);
                b.bool(p.flag("DrawHash"));
                b.bool(p.flag("FastPush"));
                b.bool(p.flag("OcarinaUnderwater"));
                b.bool(p.flag("QuestItemStorage"));
                b.bool(p.flag("CloseCows"));
                b.int(if p.flag("QuestItemStorage") { 1 } else { 0 }, 2); // QuestConsume
                b.bool(p.flag("ArrowCycling"));
                b.bool(p.flag("ArrowCycling")); // ArrowMagic
                b.bool(p.flag("ElegySpeedup"));
                b.bool(p.flag("ContinuousDekuHopping"));
                b.bool(p.flag("ProgressiveUpgrades"));
                b.bool(p.flag("TrapQuirks"));
                b.bool(p.choice("Character") == "AdultLink"); // EarlyMikau
                b.bool(p.has("StrayFairyMode", "ChestsOnly")); // FairyChests
                b.bool(p.flag("TargetHealthBar"));
                b.bool(p.flag("ClimbMostSurfaces"));
                b.bool(p.flag("FreeScarecrow"));
                b.bool(p.flag("FillWallet"));
                b.int(p.enum_value("AutoInvert") as i32, 2);
                b.bool(p.flag("HiddenRupeesSparkle"));
                b.bool(p.flag("SaferGlitches"));
                b.bool(p.flag("BombchuDrops"));
                b.bool(p.flag("KegDrops"));
                b.bool(p.flag("InstantTransform"));
                b.bool(p.flag("BombArrows"));
                b.bool(p.flag("GiantMaskAnywhere"));
                b.bool(p.flag("FewerHealthDrops"));
                b.bool(p.flag("IronGoron"));
                b.bool(p.flag("EasyFrameByFrame"));
                b.bool(p.flag("FairyMaskShimmer"));
                b.bool(p.flag("SkulltulaTokenSounds"));
                b.bool(p.flag("MinorDropSparkle"));
                b.bool(p.flag("TakeDamageOnEpona"));
                b.bool(p.flag("TakeDamageWhileShielding"));
                b.bool(p.flag("TakeDamageFromVoid"));
                b.bool(p.flag("FairyAndSkullHint") && p.custom_item_list.iter().any(|i| ocean_tokens.contains(&i.as_str())));
                b.bool(p.flag("MoonCrashErasesFile"));
                b.bool(p.has("EnemyMode", "Hyper"));
                b.finish(4)
            },
            internal_flags: {
                let mut b = BitPacker::default();
                b.bool(p.choice("LogicMode") == "Vanilla");
                for f in ["DirectToCredits", "CantFightMajora", "Fairies", "SkullTokens", "NonTransformationMasks", "TransformationMasks", "Notebook", "Hearts"] {
                    b.bool(victory(f));
                }
                b.int(remains, 3);
                b.finish(4)
            },
            speedups: {
                let mut f = 0u32;
                let mut set = |on: bool, bit: u32| f |= (on as u32) << bit;
                set(general("MilkBarPerformance"), 31);
                set(general("BlastMaskThief"), 30);
                set(general("FishermanGame"), 29);
                set(general("BoatArchery"), 28);
                set(general("HungryGoron"), 27);
                set(general("FasterBankText"), 26);
                set(p.flag("DoubleArcheryRewards"), 25);
                set(true, 24); // BankMultiRewards
                set(general("ShortChestOpening"), 23);
                set(general("EverythingElse"), 20);
                set(p.flag("OathHint"), 19);
                f | ((p.enum_value("ChestGameMinimap") as u32 & 3) << 21)
            },
            shorts: ((self.collectable_table_file as u32) << 16) | self.bank_withdraw_fee as u32,
            mmr_bytes: ((self.mask_config.npc_kafei_draw_mask as u32) << 24) | ((p.number("RequiredBossRemains") as u32 & 0xFF) << 16),
            draw_flags: {
                let m = &self.mask_config;
                let mut f = 0u32;
                let mut set = |on: bool, bit: u32| f |= (on as u32) << bit;
                set(p.flag("UpdateWorldModels"), 31);
                set(m.don_gero_goron_draw_mask, 30);
                set(m.postman_draw_hat, 29);
                set(m.draw_mask_of_truth, 28);
                set(m.draw_garo_mask, 27);
                set(m.draw_pendant_of_memories, 26);
                set(p.flag("UpdateShopAppearance"), 25);
                f
            },
            smithy: {
                let mut b = vec![0u8; 12 * 10];
                for (i, m) in self.extended.iter().flat_map(|x| x.smithy_models.iter()).enumerate() {
                    b[i * 12..i * 12 + 12].copy_from_slice(&m.to_bytes());
                }
                b
            },
        }
    }
}

/// The payload's misc config, already packed.
pub(crate) struct MiscConfig {
    flags: Vec<u8>,
    internal_flags: Vec<u8>,
    speedups: u32,
    shorts: u32,
    mmr_bytes: u32,
    draw_flags: u32,
    smithy: Vec<u8>,
}

impl MiscConfig {
    /// `MiscConfigStruct.ToBytes` (the hash is written later).
    fn to_bytes(&self, version: u32) -> Vec<u8> {
        let mut w = Vec::new();
        w.extend_from_slice(&version.to_be_bytes());
        w.extend_from_slice(&[0u8; 0x10]);
        w.extend_from_slice(&self.flags);
        if version >= 1 {
            w.extend_from_slice(&self.internal_flags);
        }
        if version >= 3 {
            for v in [self.speedups, self.shorts, self.mmr_bytes, self.draw_flags] {
                w.extend_from_slice(&v.to_be_bytes());
            }
            w.extend_from_slice(&self.smithy);
        }
        w
    }
}
