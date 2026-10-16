//! Item placement (`WriteItems`, `WriteFreeItems` and `ItemSwapUtils`):
//! starting items, the get-item table, chests, bottles and shops.

use crate::asm_config::tingle_map;
use crate::items::{self, GetItemEntry};
use crate::message::{message, MessageEntry, Msg};
use crate::plan::PlacedItem;
use crate::scene;
use crate::{BuildError, Builder};
use std::collections::{HashMap, HashSet};

const BOTTLE_CATCH_TABLE: u32 = 0xCD_7C08;

/// Upgrade groups where only the best one should be started with.
const FORBIDDEN_START_TOGETHER: [&[&str]; 7] = [
    &["ItemBow", "UpgradeBigQuiver", "UpgradeBiggestQuiver"],
    &["ItemBombBag", "UpgradeBigBombBag", "UpgradeBiggestBombBag"],
    &["UpgradeAdultWallet", "UpgradeGiantWallet", "UpgradeRoyalWallet"],
    &["StartingSword", "UpgradeRazorSword", "UpgradeGildedSword"],
    &["StartingShield", "ShopItemTradingPostShield", "ShopItemZoraShield", "UpgradeMirrorShield"],
    &["FairyMagic", "FairyDoubleMagic"],
    &["SongLullabyIntro", "SongLullaby"],
];

fn forbidden_group(item: &str) -> Option<usize> {
    FORBIDDEN_START_TOGETHER.iter().position(|g| g.contains(&item))
}

/// LINQ `GroupBy`: groups in order of first appearance.
fn group_by<'a>(list: &[&'a str], key: impl Fn(&str) -> Option<usize>) -> Vec<(Option<usize>, Vec<&'a str>)> {
    let mut groups: Vec<(Option<usize>, Vec<&'a str>)> = Vec::new();
    for &item in list {
        let k = key(item);
        match groups.iter_mut().find(|(gk, _)| *gk == k) {
            Some((_, g)) => g.push(item),
            None => groups.push((k, vec![item])),
        }
    }
    groups
}

impl Builder<'_> {
    pub(crate) fn is_location_junk(&self, location: &str) -> bool {
        self.junk_locations.contains(location)
    }

    /// The item placed at a location.
    pub(crate) fn placed_at(&self, location: &str) -> Option<&PlacedItem> {
        self.plan.items.iter().find(|i| i.location.as_deref() == Some(location))
    }

    /// The plan entry with this original id (`ItemList[item]`).
    pub(crate) fn item_object(&self, item: &str) -> Option<&PlacedItem> {
        self.plan.items.iter().find(|i| i.id == item)
    }

    pub(crate) fn write_items(&mut self) -> Result<(), BuildError> {
        if self.plan.choice("LogicMode") == "Vanilla" {
            // Items stay where they are; only the starting items are given.
            let mut free: Vec<String> = ["FairyMagic", "MaskDeku", "ItemOcarina", "SongTime", "SongHealing", "StartingSword", "StartingShield", "StartingHeartContainer1", "StartingHeartContainer2"]
                .iter()
                .map(|s| s.to_string())
                .collect();
            // Without the giants' cutscene, nothing teaches the Oath.
            if self.cutscene_general("EverythingElse") {
                free.push("SongOath".into());
            }
            return self.write_free_items(&free);
        }
        let starting_slots = ["MaskDeku", "SongHealing", "StartingSword", "StartingShield", "StartingHeartContainer1", "StartingHeartContainer2"];
        let free: Vec<String> = starting_slots
            .iter()
            .filter(|l| !self.is_location_junk(l))
            .map(|l| self.placed_at(l).map(|p| p.item.clone()).ok_or_else(|| BuildError::Data(format!("nothing placed at {l}"))))
            .collect::<Result<_, _>>()?;
        self.write_free_items(&free)?;

        self.replace_get_item_table()?;
        self.init_items()?;

        // Extended objects (recoloured models, smithy models).
        let any_category = |c: &str| self.plan.custom_item_list.iter().any(|i| items::item_category(i) == Some(c));
        let (fairies, skulltulas) = (any_category("StrayFairies"), any_category("SkulltulaTokens"));
        let display = |p: &PlacedItem| p.mimic.as_ref().map(|m| m.item.clone()).unwrap_or_else(|| p.item.clone());
        let smithy1 = self.placed_at("UpgradeRazorSword").map(display).unwrap_or_default();
        let smithy2 = self.placed_at("UpgradeGildedSword").map(display).unwrap_or_default();
        self.create_extended_objects(&smithy1, &smithy2, fairies, skulltulas, self.plan.flag("ProgressiveUpgrades"))?;

        if self.plan.flag("FixEponaSword") {
            self.apply_mod("mods", "fix_epona")?;
        }
        if self.plan.flag("PreventDowngrades") {
            self.apply_mod("mods", "fix_downgrades")?;
        }
        if any_category("Milk") {
            self.apply_mod("mods", "fix_cow_bottle_check")?;
        }
        self.apply_mod("mods", "update_trade_scrubs")?;

        self.mmr_config.rupee_repeatable_locations.clear();
        self.mmr_config.items_to_return_ids.clear();
        let mut new_messages = Vec::new();
        const KILL_BOSSES: [&str; 4] = ["OtherKillOdolwa", "OtherKillGoht", "OtherKillGyorg", "OtherKillTwinmold"];
        let placed: Vec<PlacedItem> = self.plan.items.clone();
        for io in &placed {
            let Some(location) = io.location.as_deref() else { continue };
            if items::is_entrance(&io.item) || KILL_BOSSES.contains(&io.item.as_str()) {
                continue;
            }
            if items::is_bottle_catch_content(&io.item) {
                self.write_new_bottle(location, &io.item)?;
                continue;
            }
            let item = io.item.as_str();
            let name = items::name(item).unwrap_or("");
            let category = items::item_category(item);
            let mut override_chest: Option<String> = None;
            if matches!(category, Some("StrayFairies" | "SkulltulaTokens" | "Frogs")) && !self.item_is_used(item) {
                override_chest = Some("SmallWooden".into());
            }
            if (name.contains("Bombchu") || name.contains("Shield")) && self.item_is_used(item) {
                override_chest = Some(if items::is_temporary(self.plan, item) { "SmallGold" } else { "LargeGold" }.into());
            }
            if name.contains("Compass") && self.plan.has("DungeonNavigationMode", "CompassRevealsBoss") {
                override_chest = Some("LargeGold".into());
            }
            if name.contains("Map") && items::classic_category(item) == Some("DungeonItems") && self.plan.has("DungeonNavigationMode", "MapRevealsLocation") {
                override_chest = Some("LargeGold".into());
            }
            if self.plan.has("VictoryMode", "Hearts") && matches!(category, Some("PiecesOfHeart" | "HeartContainers")) {
                override_chest = Some("LargeGold".into());
            }
            if self.plan.has("VictoryMode", "Fairies") && items::in_range(item, "CollectibleStrayFairyWoodfall1", "CollectibleStrayFairyStoneTower15") {
                override_chest = Some("LargeGold".into());
            }
            if self.plan.has("VictoryMode", "SkullTokens") && category == Some("SkulltulaTokens") {
                override_chest = Some("LargeGold".into());
            }
            let chest = io.mimic.as_ref().and_then(|m| m.chest_type.clone()).or(override_chest);
            self.write_new_item(io, location, chest, &mut new_messages)?;
        }

        let loc = |b: &Self, item: &str| -> u16 {
            match b.item_object(item) {
                Some(io) if io.item == item => io.location.as_deref().and_then(items::get_item_index).unwrap_or(0),
                _ => 0,
            }
        };
        let c = MmrLocations {
            red_potion: loc(self, "ItemBottleWitch"),
            gold_dust: loc(self, "ItemBottleGoronRace"),
            milk: loc(self, "ItemBottleAliens"),
            chateau: loc(self, "ItemBottleMadameAroma"),
            kokiri: loc(self, "StartingSword"),
            razor: loc(self, "UpgradeRazorSword"),
            gilded: loc(self, "UpgradeGildedSword"),
            magic_small: loc(self, "FairyMagic"),
            magic_large: loc(self, "FairyDoubleMagic"),
            wallet_adult: loc(self, "UpgradeAdultWallet"),
            wallet_giant: loc(self, "UpgradeGiantWallet"),
            wallet_royal: loc(self, "UpgradeRoyalWallet"),
            bomb_small: loc(self, "ItemBombBag"),
            bomb_big: loc(self, "UpgradeBigBombBag"),
            bomb_biggest: loc(self, "UpgradeBiggestBombBag"),
            quiver_small: loc(self, "ItemBow"),
            quiver_large: loc(self, "UpgradeBigQuiver"),
            quiver_largest: loc(self, "UpgradeBiggestQuiver"),
            lullaby: loc(self, "SongLullaby"),
            lullaby_intro: loc(self, "SongLullabyIntro"),
        };
        c.apply(&mut self.mmr_config);

        self.write_item_texts(&mut new_messages)?;
        self.write_prices(&mut new_messages)?;
        self.write_gibdo_requirements()?;
        self.messages.update_all(new_messages);
        self.apply_mod("mods", "fix_shop_curiosity_bigbombbag")
    }

    /// Starting items (`WriteFreeItems`).
    fn write_free_items(&mut self, given: &[String]) -> Result<(), BuildError> {
        let mut starting: Vec<(u32, u8)> = Vec::new();
        let mut put = |addr: u32, value: u8, add: bool| match starting.iter_mut().find(|(a, _)| *a == addr) {
            Some((_, v)) => *v = if add { v.wrapping_add(value) } else { *v | value },
            None => starting.push((addr, if add { value } else { value })),
        };
        if self.plan.flag("EnableSunsSong") {
            put(0xC5CE71, 0x02, false);
        }
        let mut list: Vec<&str> = given.iter().map(String::as_str).filter(|i| *i != "RecoveryHeart").collect();
        list.extend(self.plan.custom_starting_item_list.iter().map(String::as_str));
        list.extend(self.plan.blitz_extra_items.iter().flatten().map(String::as_str));
        list.extend(self.plan.random_starting_items.iter().flatten().map(String::as_str));
        let mut seen = HashSet::new();
        list.retain(|i| seen.insert(*i));

        list.push("StartingHeartContainer1");
        let is_piece = |i: &&str| items::name(i) == Some("Piece of Heart");
        while list.iter().filter(|i| is_piece(i)).count() >= 4 {
            list.push("StartingHeartContainer1");
            for _ in 0..4 {
                let at = list.iter().position(|i| is_piece(&i)).unwrap();
                list.remove(at);
            }
        }
        if self.plan.flag("ProgressiveUpgrades") {
            list = group_by(&list, forbidden_group)
                .into_iter()
                .flat_map(|(k, g)| match k {
                    Some(k) if !FORBIDDEN_START_TOGETHER[k].contains(&"StartingShield") => vec![FORBIDDEN_START_TOGETHER[k][g.len() - 1]],
                    _ => g,
                })
                .collect();
        }
        list = group_by(&list, forbidden_group)
            .into_iter()
            .flat_map(|(k, g)| match k {
                Some(k) => {
                    let key = FORBIDDEN_START_TOGETHER[k];
                    let best = g.iter().copied().max_by_key(|i| key.iter().position(|x| x == i)).unwrap();
                    // OrderByDescending is stable: the first of equal ranks wins.
                    let rank = key.iter().position(|x| *x == best);
                    vec![g.into_iter().find(|i| key.iter().position(|x| x == i) == rank).unwrap()]
                }
                None => g,
            })
            .collect();

        let cfg = &mut self.mmr_config;
        cfg.extra_starting_maps = 0;
        cfg.extra_starting_swamp_skull_tokens = 0;
        cfg.extra_starting_ocean_skull_tokens = 0;
        cfg.extra_starting_item_ids.clear();
        for item in &list {
            if let Some(map) = items::starting_tingle_map(item) {
                cfg.extra_starting_maps |= tingle_map(map);
                continue;
            }
            if items::has_starting_item_skull(item) {
                if items::range_from("CollectibleOceanSpiderToken1", 30).contains(item) {
                    cfg.extra_starting_ocean_skull_tokens += 1;
                } else if items::range_from("CollectibleSwampSpiderToken1", 30).contains(item) {
                    cfg.extra_starting_swamp_skull_tokens += 1;
                } else {
                    return Err(BuildError::Data(format!("invalid starting token {item}")));
                }
                continue;
            }
            let ids = items::starting_item_ids(item);
            if !ids.is_empty() {
                cfg.extra_starting_item_ids.extend(ids);
                continue;
            }
            let writes = items::starting_item_writes(item);
            if writes.is_empty() && self.plan.choice("StartingItemMode") != "None" {
                return Err(BuildError::Data(format!("invalid starting item {item}")));
            }
            for (addr, value, add) in writes {
                put(addr, value, add);
            }
        }
        for (addr, value) in starting {
            self.rom.write_u8(addr, value)?;
        }
        if list.iter().filter(|i| items::name(i) == Some("Heart Container")).count() == 1 {
            // Lower the low-health beep threshold.
            self.rom.write_u8(0x00B97E8F, 0x0C)?;
        }
        Ok(())
    }

    /// Install the randomizer's get-item, chest and collectable tables
    /// (`ItemSwapUtils.ReplaceGetItemTable`).
    fn replace_get_item_table(&mut self) -> Result<(), BuildError> {
        self.apply_mod("mods", "replace_gi_table")?;
        let last_file = self.rom.files.len() as u32 - 1;
        let gi = self.rom.append_file(crate::resources::mods("gi_table").unwrap().to_vec(), false);
        self.get_item_table = self.rom.files[gi].addr;
        self.rom.write_u32(0xB3C000 + 0x13B6C4, last_file + 1)?;
        self.apply_mod("mods", "update_chests")?;
        self.rom.append_file(crate::resources::mods("chest_table").unwrap().to_vec(), false);
        self.rom.write_u32(0xB3C000 + 0x13B6C0, last_file + 2)?;
        self.rom.append_file(crate::resources::mods("collectable_table").unwrap().to_vec(), false);
        self.collectable_table_file = (last_file + 3) as usize;
        self.apply_mod("mods", "standing_hearts")?;
        self.apply_mod("mods", "fix_item_checks")?;
        scene::reset_flag_mask(&mut self.rom)?;
        for gi in [0x5B, 0x91, 0x92, 0x93] {
            scene::update_flag_mask(&mut self.rom, gi)?;
        }
        Ok(())
    }

    /// Read the get-item and bottle tables (`ItemSwapUtils.InitItems`).
    fn init_items(&mut self) -> Result<(), BuildError> {
        let f = self.rom.index_of(self.get_item_table).unwrap();
        let data = self.rom.file(f)?.clone();
        self.get_items = (0..data.len() / 8).map(|i| ((i + 1) as u16, GetItemEntry::read(&data[i * 8..i * 8 + 8]))).collect();
        let f = self.rom.index_of(BOTTLE_CATCH_TABLE).unwrap();
        self.rom.open(f)?;
        let base = (BOTTLE_CATCH_TABLE - self.rom.files[f].addr) as usize;
        let data = &self.rom.files[f].data;
        let mut bottles = HashMap::new();
        for v in &items::table().values {
            for idx in items::bottle_indices(&v.name).unwrap_or_default() {
                let o = idx as usize * 6 + base;
                bottles.insert(idx, [data[o + 3], data[o + 4], data[o + 5]]);
            }
        }
        self.bottles = bottles;
        Ok(())
    }

    fn write_new_bottle(&mut self, location: &str, item: &str) -> Result<(), BuildError> {
        let f = self.rom.index_of(BOTTLE_CATCH_TABLE).unwrap();
        self.rom.open(f)?;
        let base = (BOTTLE_CATCH_TABLE - self.rom.files[f].addr) as usize;
        let source = self.bottles[&items::bottle_indices(item).unwrap()[0]];
        for idx in items::bottle_indices(location).unwrap_or_default() {
            let o = idx as usize * 6 + base;
            self.rom.files[f].data[o + 3..o + 6].copy_from_slice(&source);
        }
        Ok(())
    }

    /// Whether logic needs an item anywhere (directly or through helpers).
    fn item_is_used(&self, item: &str) -> bool {
        fn used(b: &Builder, item: &str, path: &mut Vec<String>) -> bool {
            if path.iter().any(|p| p == item) {
                return false;
            }
            path.push(item.to_string());
            let users: Vec<&crate::plan::LogicEntry> = b
                .plan
                .logic
                .iter()
                .filter(|il| !b.is_location_junk(&il.id))
                .filter(|il| il.required.iter().flatten().any(|r| r == item) || il.conditional.iter().flatten().any(|c| c.iter().any(|r| r == item)))
                .collect();
            let result = users.iter().any(|il| !il.is_fake_item) || users.iter().any(|il| used(b, &il.id, path));
            path.pop();
            result
        }
        used(self, item, &mut Vec::new())
    }

    /// Put an item at a location (`ItemSwapUtils.WriteNewItem`).
    fn write_new_item(&mut self, io: &PlacedItem, location: &str, override_chest: Option<String>, new_messages: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        let item = io.item.as_str();
        let display_item = io.mimic.as_ref().map(|m| m.item.as_str()).unwrap_or(item);
        if !io.is_randomized {
            let indices = items::collectable_indices(location);
            if !indices.is_empty() {
                let table = &mut self.rom.files[self.collectable_table_file].data;
                for i in indices {
                    table[i as usize * 2..i as usize * 2 + 2].copy_from_slice(&[0, 0]);
                }
                return Ok(());
            }
        }
        let f = self.rom.index_of(self.get_item_table).unwrap();
        self.rom.open(f)?;
        let base = (self.get_item_table - self.rom.files[f].addr) as usize;
        let gi = items::get_item_index(location).ok_or_else(|| BuildError::Data(format!("{location} has no get-item index")))?;
        let offset = (gi as usize - 1) * 8 + base;

        // The old code edits shared table entries in place, so later items
        // see these changes; `source` says which entry to write back to.
        let (mut entry, source) = if !io.is_randomized && items::is_nullable(location) {
            (GetItemEntry::default(), None)
        } else if items::is_exclusive(item) {
            (items::exclusive_entry(item).unwrap(), None)
        } else {
            let index = items::get_item_index(item).ok_or_else(|| BuildError::Data(format!("{item} has no get-item index")))?;
            (self.get_items[&index], Some(index))
        };
        if !io.is_randomized && items::item_category(item) == Some("NotebookEntries") {
            entry.message = 0;
        }
        if matches!(gi, 0x80 | 0x81 | 0x84 | 0x88 | 0x8A | 0xAB) {
            self.mask_config.update(location, display_item);
        }
        if let Some(g) = self.extended.as_ref().unwrap().resolve(&entry) {
            entry.object = g.object;
            entry.index = g.graphic;
        }
        if let Some(index) = source {
            self.get_items.insert(index, entry);
        }
        self.rom.files[f].data[offset..offset + 8].copy_from_slice(&entry.to_bytes());

        let refill = match item {
            "ItemBottleMadameAroma" => Some(0x91u16),
            "ItemBottleAliens" => Some(0x92),
            _ => None,
        };
        if let Some(r) = refill {
            let mut e = self.get_items[&r];
            if let Some(g) = self.extended.as_ref().unwrap().resolve(&e) {
                e.object = g.object;
                e.index = g.graphic;
            }
            self.get_items.insert(r, e);
            let o = (r as usize - 1) * 8 + base;
            self.rom.files[f].data[o..o + 8].copy_from_slice(&e.to_bytes());
        }

        if items::is_rupee_repeatable(location) {
            self.mmr_config.rupee_repeatable_locations.push(gi);
        }
        let mut repeatable = items::is_repeatable(self.plan, item) || (!self.plan.flag("PreventDowngrades") && items::is_downgradable(item));
        if items::is_returnable(self.plan, item) {
            repeatable = false;
            self.mmr_config.items_to_return_ids.push(gi);
        }
        if location == "ItemOcarina" && self.logically_junk.contains(item) {
            repeatable = false;
        }
        if !repeatable {
            scene::update_flag_mask(&mut self.rom, gi as u32)?;
        }

        let chest = if !self.plan.flag("UpdateChests") { items::chest_type(location).map(String::from) } else { override_chest };
        self.update_chest(location, item, chest)?;

        if self.plan.flag("UpdateShopAppearance") {
            self.update_shop(io, location, display_item, new_messages)?;
        }
        if io.is_randomized {
            for (hack, only_if_different) in items::hack_contents(location) {
                if location == item && only_if_different {
                    continue;
                }
                self.apply_mod("mods", hack)?;
            }
        }
        Ok(())
    }

    fn update_chest(&mut self, location: &str, item: &str, override_chest: Option<String>) -> Result<(), BuildError> {
        let chest_name = override_chest.or_else(|| items::chest_type(item).map(String::from)).unwrap_or_else(|| "SmallWooden".into());
        let chest_type = items::chest_type_value(&chest_name);
        if let Some((addresses, appearance)) = items::chest(location) {
            for addr in addresses {
                let mut v = self.rom.read_u8(addr)?;
                v &= 0x0F;
                let t = ((chest_type << 2) + appearance as u8) << 4;
                v |= t;
                self.rom.write_u8(addr, v)?;
            }
        }
        if items::is_grotto_chest(location) {
            let grotto = items::region_reference(location).ok_or_else(|| BuildError::Data(format!("{location}: grotto without region")))?;
            let placed = self.item_object(grotto).and_then(|io| io.location.clone()).unwrap_or_else(|| grotto.to_string());
            let entrance = items::entrances(&placed).first().cloned().ok_or_else(|| BuildError::Data(format!("{placed} is not an entrance")))?;
            let new_type = chest_type << 5;
            for (scene_id, setup, room, actor) in items::exit_actor_params(&entrance) {
                self.write_scene_actor_params(scene_id, setup, room, actor, 0x55, new_type as u16, 0x60)?;
            }
        }
        Ok(())
    }

    /// Change an actor's parameters (`EntranceUtils.WriteSceneActorParams`,
    /// without rotation changes).
    pub(crate) fn write_scene_actor_params(&mut self, scene_number: u8, setup: usize, room: usize, actor: usize, actor_id: u16, param: u16, mask: u16) -> Result<(), BuildError> {
        let scene = self.scenes.iter().find(|s| s.number == scene_number).ok_or_else(|| BuildError::Data(format!("no scene {scene_number}")))?;
        let room = &scene.setups[setup].rooms[room];
        let Some(list) = room.actor_list_address else { return Ok(()) };
        let file = room.file;
        let at = list + actor * 0x10;
        let data = self.rom.file(file)?;
        let id = u16::from_be_bytes([data[at], data[at + 1]]);
        if id & 0x1FFF != actor_id {
            return Err(BuildError::Data("attempt to write to the wrong actor".into()));
        }
        let old = u16::from_be_bytes([data[at + 0xE], data[at + 0xF]]);
        let new = (old & !mask) | (param & mask);
        data[at + 0xE..at + 0x10].copy_from_slice(&new.to_be_bytes());
        Ok(())
    }

    /// Shop text for a location sold in shops (`ItemSwapUtils.UpdateShop`).
    fn update_shop(&mut self, io: &PlacedItem, location: &str, display_item: &str, new_messages: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        let gi = items::get_item_index(location).unwrap();
        for (address, keeper) in items::shop_inventories(location) {
            let message_id = self.rom.read_u16(address + 0x0A)?;
            let old = self.messages.get(message_id + 1).ok_or_else(|| BuildError::Data(format!("missing shop message {:#X}", message_id + 1)))?;
            let header = old.header.unwrap_or(crate::message::DEFAULT_HEADER);
            let cost = u16::from_be_bytes([header[5], header[6]]);
            let item_cost = format!("{cost} Rupee{}", if cost != 1 { "s" } else { "" });
            let max_name = 35 - format!(": {item_cost}").len();
            let mut name = display_name(io).to_string();
            if name.encode_utf16().count() > max_name {
                let units: Vec<u16> = name.encode_utf16().take(max_name - 3).collect();
                name = String::from_utf16_lossy(&units) + "...";
            }
            new_messages.push(message(message_id, None, |it| {
                it.color(crate::message::RED, |it| {
                    it.runtime_start(3, gi).text(&name).runtime_generic_stop();
                    it.text(": ").text(&item_cost).new_line();
                });
                it.runtime_wrap(|it| {
                    runtime_item_description(it, display_item, keeper, gi);
                });
                it.disable_text_box_close().end_final_text_box();
            }));
            new_messages.push(message(message_id + 1, None, |it| {
                it.runtime_start(3, gi).text(&name).runtime_generic_stop();
                it.text(": ").text(&item_cost).new_line();
                it.text(" ").new_line();
                it.start_green().two_choices();
                it.text("I'll buy ");
                it.runtime_start(6, gi).text(pronoun(display_item)).runtime_generic_stop();
                it.new_line().text("No thanks").end_final_text_box();
            }));
        }
        Ok(())
    }
}

/// The old `ItemObject.DisplayName`.
pub(crate) fn display_name(io: &PlacedItem) -> &str {
    io.mimic
        .as_ref()
        .and_then(|m| m.fake_name.as_deref().or_else(|| items::name(&m.item)))
        .or_else(|| items::name(&io.item))
        .unwrap_or("")
}

/// Digits in an item's name ("10 Bombs" → "10").
fn amount(item: &str) -> String {
    items::name(item).unwrap_or("").chars().filter(char::is_ascii_digit).collect()
}

/// `MessageUtils.GetPronounOrAmount`.
pub(crate) fn pronoun_or_amount(item: &str) -> String {
    let texts = items::shop_text(item).unwrap_or_default();
    let n = amount(item);
    if texts.is_multiple {
        if n.trim().is_empty() { " it".into() } else { format!(" {n}") }
    } else if texts.is_definite {
        " it".into()
    } else {
        " one".into()
    }
}

/// `MessageUtils.GetVerb`.
pub(crate) fn verb(item: &str) -> &'static str {
    let multiple = items::shop_text(item).map(|s| s.is_multiple).unwrap_or(false);
    if multiple && !amount(item).trim().is_empty() {
        "are"
    } else {
        "is"
    }
}

/// `MessageUtils.GetAlternateName`: drop counts ("10 Bombs" -> "Bombs").
pub(crate) fn alternate_name(name: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = name.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j < chars.len() && chars[j] == ' ' {
                i = j + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `MessageUtils.GetPronoun`.
pub(crate) fn pronoun(item: &str) -> &'static str {
    let multiple = items::shop_text(item).map(|s| s.is_multiple).unwrap_or(false);
    if multiple && !amount(item).trim().is_empty() {
        "them"
    } else {
        "it"
    }
}

/// `RuntimeItemDescription`: the shop description, filled in at runtime.
pub(crate) fn runtime_item_description(it: &mut Msg, item: &str, keeper: &str, gi: u16) {
    let texts = items::shop_text(item).unwrap_or_default();
    let description = texts.for_keeper(keeper).unwrap_or(texts.default.as_str()).to_string();
    let units: Vec<u16> = description.encode_utf16().collect();
    let marker = [0x09u16, 0x01, 0x00, 0x00];
    if units.windows(4).any(|w| w == marker) {
        let mut out = Vec::new();
        let mut i = 0;
        while i < units.len() {
            if units[i..].starts_with(&marker) {
                out.extend([0x09, 0x01, gi >> 8, gi & 0xFF]);
                i += 4;
            } else {
                out.push(units[i]);
                i += 1;
            }
        }
        it.units(&out);
        return;
    }
    it.runtime_start(4, gi).units(&units).runtime_generic_stop();
}

/// The `MMRConfig` location ids set after placing items.
struct MmrLocations {
    red_potion: u16,
    gold_dust: u16,
    milk: u16,
    chateau: u16,
    kokiri: u16,
    razor: u16,
    gilded: u16,
    magic_small: u16,
    magic_large: u16,
    wallet_adult: u16,
    wallet_giant: u16,
    wallet_royal: u16,
    bomb_small: u16,
    bomb_big: u16,
    bomb_biggest: u16,
    quiver_small: u16,
    quiver_large: u16,
    quiver_largest: u16,
    lullaby: u16,
    lullaby_intro: u16,
}

impl MmrLocations {
    fn apply(self, c: &mut crate::asm_config::MmrConfig) {
        c.location_bottle_red_potion = self.red_potion;
        c.location_bottle_gold_dust = self.gold_dust;
        c.location_bottle_milk = self.milk;
        c.location_bottle_chateau = self.chateau;
        c.location_sword_kokiri = self.kokiri;
        c.location_sword_razor = self.razor;
        c.location_sword_gilded = self.gilded;
        c.location_magic_small = self.magic_small;
        c.location_magic_large = self.magic_large;
        c.location_wallet_adult = self.wallet_adult;
        c.location_wallet_giant = self.wallet_giant;
        c.location_wallet_royal = self.wallet_royal;
        c.location_bomb_bag_small = self.bomb_small;
        c.location_bomb_bag_big = self.bomb_big;
        c.location_bomb_bag_biggest = self.bomb_biggest;
        c.location_quiver_small = self.quiver_small;
        c.location_quiver_large = self.quiver_large;
        c.location_quiver_largest = self.quiver_largest;
        c.location_lullaby = self.lullaby;
        c.location_lullaby_intro = self.lullaby_intro;
    }
}
