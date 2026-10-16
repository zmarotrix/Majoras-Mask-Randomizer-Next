//! Entrance randomization (`WriteEntrances`, with `EntranceSwapUtils` and
//! `EntranceUtils`).

use crate::enums;
use crate::items;
use crate::stages::hacks::addresses;
use crate::{BuildError, Builder};
use serde_json::Value;

/// Scenes whose exits change together (the same area in another state).
const SCENE_SYNC: [(u8, u8); 4] = [(69, 0), (80, 90), (93, 94), (77, 72)];

/// Per dungeon: the flag and mask that say it's cleared, and its scene.
const CLEAR_FLAGS: [u16; 4] = [0x57C, 0x589, 0x59C, 0x59F];
const CLEAR_MASKS: [u16; 4] = [0x02, 0x80, 0x20, 0x80];
const CLEAR_SCENES: [u8; 4] = [0x1F, 0x44, 0x36, 0x5F];

fn num(v: &Value) -> i64 {
    v.as_i64().unwrap_or(0)
}

fn spawn_id(entrance: &str) -> Option<u16> {
    enums::entrances().attr(entrance, "Spawn").and_then(|a| a.get("SpawnId").as_u64()).map(|v| v as u16)
}

/// The entrance items of one type, in `Item` order.
fn entrances_of_type(kind: &str) -> Vec<String> {
    items::table()
        .values
        .iter()
        .filter(|v| v.attributes.iter().any(|a| a.kind == "EntranceType" && a.get("Type").as_str() == Some(kind)))
        .map(|v| v.name.clone())
        .collect()
}

/// RGBA5551 to colour (`ColorUtils.FromRGBA5551`): (a, r, g, b).
fn from_5551(c: u16) -> [i32; 4] {
    let ch = |v: u16| (v as f32 * 255.0f32 / 31.0f32) as i32;
    [((c & 1) * 255) as i32, ch((c & 0xF800) >> 11), ch((c & 0x7C0) >> 6), ch((c & 0x3E) >> 1)]
}

fn to_5551(c: [i32; 4]) -> u16 {
    let ch = |v: i32| (v as u16 >> 3) & 0x1F;
    (ch(c[1]) << 11) | (ch(c[2]) << 6) | (ch(c[3]) << 1) | u16::from(c[0] > 0x7F)
}

/// Reduce a 16-bit texture to a palette of at most 256 colours, merging the
/// least used colour into its nearest neighbour each time, as the old code
/// did. Returns (CI8 pixels, palette).
fn to_ci8(pixels: &mut [u16]) -> (Vec<u8>, Vec<u16>) {
    // (colour, count) in first-seen order, like the old dictionary.
    let mut palette: Vec<(u16, usize)> = Vec::new();
    for &p in pixels.iter() {
        match palette.iter_mut().find(|(c, _)| *c == p) {
            Some(e) => e.1 += 1,
            None => palette.push((p, 1)),
        }
    }
    while palette.len() > 256 {
        let least = (0..palette.len()).min_by_key(|&i| palette[i].1).unwrap();
        let (colour, count) = palette.remove(least);
        let current = from_5551(colour);
        let distance = |m: [i32; 4]| (0..4).map(|k| (current[k] - m[k]) * (current[k] - m[k])).sum::<i32>();
        let mut best = 0;
        let mut shortest = i32::MAX;
        for (i, (c, _)) in palette.iter().enumerate() {
            let d = distance(from_5551(*c));
            if d < shortest {
                best = i;
                shortest = d;
            }
        }
        let nearest = to_5551(from_5551(palette[best].0));
        let target = palette.iter_mut().find(|(c, _)| *c == nearest).expect("palette round trip");
        target.1 += count;
        for p in pixels.iter_mut().filter(|p| **p == colour) {
            *p = nearest;
        }
    }
    let colours: Vec<u16> = palette.iter().map(|(c, _)| *c).collect();
    let ci8 = pixels.iter().map(|p| colours.iter().position(|c| c == p).map_or(0xFF, |i| i as u8)).collect();
    (ci8, colours)
}

impl Builder<'_> {
    pub(crate) fn write_entrances(&mut self) -> Result<(), BuildError> {
        if self.plan.choice("LogicMode") == "Vanilla" || self.plan.enum_value("EntranceMode") == 0 {
            return Ok(());
        }
        let mode = |f: &str| self.plan.has("EntranceMode", f);
        let mut list: Vec<String> = Vec::new();
        if mode("DungeonEntrances") {
            list.extend(entrances_of_type("Dungeon"));
            list.extend(entrances_of_type("DungeonExit"));
        }
        if mode("BossRooms") {
            list.extend(entrances_of_type("Boss"));
            list.extend(entrances_of_type("DungeonExit"));
        }
        if mode("Grottos") {
            list.extend(entrances_of_type("Grotto"));
            // A Japanese-version grotto entrance becomes the Deku playground's.
            self.rom.write_u16(0xD5A95A, spawn_id("EntranceGrottoDekuPlayground").unwrap())?;
        }
        if mode("SimpleInteriors") {
            list.extend(entrances_of_type("Interior"));
        }
        let mut seen = std::collections::HashSet::new();
        list.retain(|e| seen.insert(e.clone()));

        for entrance in &list {
            let spawns = items::entrances(entrance);
            let location = self.item_object(entrance).and_then(|o| o.location.clone()).ok_or_else(|| BuildError::Data(format!("{entrance} wasn't placed")))?;
            let exits = items::entrances(&location);
            self.write_new_entrance(&exits[0], &spawns[0])?;
            if exits.len() > 1 && spawns.len() > 1 {
                self.write_new_entrance(&spawns[1], &exits[1])?;
            }
        }

        if !mode("DungeonEntrances") && !mode("BossRooms") {
            return Ok(());
        }
        // Boss warps, the giants' cutscene and "cleared" checks follow the
        // dungeons to where they now are.
        let clears = ["AreaWoodFallTempleClear", "AreaSnowheadTempleClear", "AreaStoneTowerClear", "AreaGreatBayTempleClear"];
        let check = addresses(crate::resources::addresses("d_check").unwrap());
        let flag_load = addresses(crate::resources::addresses("dc_flagload").unwrap());
        let flag_mask = addresses(crate::resources::addresses("dc_flagmask").unwrap());
        let giants = addresses(crate::resources::addresses("d_giants_cs").unwrap());
        let mut li = [0x24, 0x02, 0x00, 0x00];
        let mut addiu = [0x24, 0x01, 0x00, 0x00];
        for (i, clear) in clears.iter().enumerate() {
            let exit = self.placed_at(clear).map(|p| p.item.clone()).unwrap_or_default();
            let new = clears.iter().position(|c| *c == exit).ok_or_else(|| BuildError::Data("dungeon clears are mixed up".into()))?;
            li[3] = new as u8;
            self.write_rom_addr(&check[i], &mut li)?;
            addiu[3] = CLEAR_SCENES[i];
            self.write_rom_addr(&giants[new], &mut addiu)?;
            self.write_rom_addr(&flag_load[i], &mut CLEAR_FLAGS[new].to_be_bytes())?;
            self.write_rom_addr(&flag_mask[i], &mut CLEAR_MASKS[new].to_be_bytes())?;
        }

        if mode("BossRooms") {
            let bosses = ["AreaOdolwasLair", "AreaGohtsLair", "AreaGyorgsLair", "AreaTwinmoldsLair"];
            let doors = addresses(crate::resources::addresses("d_boss_door").unwrap());
            let warps = addresses(crate::resources::addresses("d_boss_warp").unwrap());
            let door_values: [[u8; 3]; 4] = [[0x00, 0x1F, 0x01], [0x00, 0x44, 0x02], [0x00, 0x5F, 0x03], [0x00, 0x36, 0x04]];
            for (i, boss) in bosses.iter().enumerate() {
                let new = self.item_object(boss).and_then(|o| o.location.clone()).unwrap_or_else(|| boss.to_string());
                let at = bosses.iter().position(|b| *b == new).ok_or_else(|| BuildError::Data(format!("{boss} went somewhere odd")))?;
                self.write_rom_addr(&doors[at], &mut door_values[i].clone())?;
                self.write_rom_addr(&warps[at], &mut [(i + 2) as u8])?;
            }
            // Stone Tower's boss door shows the boss now behind it.
            let at_stt = self.placed_at("AreaTwinmoldsLair").map(|p| p.item.clone()).unwrap_or_else(|| "AreaTwinmoldsLair".into());
            if at_stt != "AreaTwinmoldsLair" {
                let index = bosses.iter().position(|b| *b == at_stt).unwrap_or(3);
                let offsets = [0x5BA0u32, 0x5C0, 0x4BA0, 0x3BA0];
                let texture = self.rom.bytes_mut(0x012F_8000 + offsets[index], 0x1000)?.to_vec();
                let mut pixels: Vec<u16> = texture.chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
                let (ci8, palette) = to_ci8(&mut pixels);
                let palette_bytes: Vec<u8> = palette.iter().flat_map(|c| c.to_be_bytes()).collect();
                const SCENE: u32 = 0x020C_F000;
                for (room_addr, image_at, pointer_at, pointer, palette_at, room) in
                    [(0x0211_D000u32, 0x4428u32, 0x3D4usize, [0x03, 0x00, 0x4C, 0x50], 0x4C50usize, 8u32), (0x0212_B000, 0x4220, 0x2434, [0x03, 0x00, 0x4A, 0x30], 0x4A30, 10)]
                {
                    self.rom.write(room_addr + image_at, &ci8)?;
                    let f = self.rom.index_of(room_addr).unwrap();
                    let data = self.rom.file(f)?;
                    data[pointer_at..pointer_at + 4].copy_from_slice(&pointer);
                    data.splice(palette_at..palette_at, palette_bytes.iter().copied());
                    let file = &mut self.rom.files[f];
                    file.end = file.addr + file.data.len() as u32;
                    let end = file.end;
                    self.rom.write_u32(SCENE + 0x178 + 8 * room + 4, end)?;
                }
            }
        }
        Ok(())
    }

    /// Make an exit lead to a spawn (`EntranceSwapUtils.WriteNewEntrance`).
    fn write_new_entrance(&mut self, exit: &str, spawn: &str) -> Result<(), BuildError> {
        let e = enums::entrances();
        let actor_params: Vec<_> = e.attrs(exit, "ExitActorParams").collect();
        if let Some(id) = spawn_id(spawn) {
            if let Some(p) = e.attr(exit, "ExitPolygonType") {
                let indices: Vec<u16> = p.get("NewExitIndices").as_array().map(|a| a.iter().map(|v| num(v) as u16).collect()).unwrap_or_default();
                self.write_polygon_type_exit(num(p.get("SceneId")) as u8, num(p.get("PolygonType")) as usize, &indices, id)?;
            }
            let synced = |scene: u8| SCENE_SYNC.iter().find(|(a, _)| *a == scene).map(|(_, b)| *b);
            for a in e.attrs(exit, "Exit") {
                let (scene, index) = (num(a.get("SceneId")) as u8, num(a.get("ExitIndex")) as usize);
                self.write_scene_exits(scene, index, id)?;
                if let Some(other) = synced(scene) {
                    self.write_scene_exits(other, index, id)?;
                }
            }
            for a in e.attrs(exit, "ExitCutscene") {
                let (scene, setup, cutscene) = (num(a.get("SceneId")) as u8, num(a.get("SetupIndex")) as usize, num(a.get("CutsceneIndex")) as usize);
                self.write_cutscene_exits(scene, setup, cutscene, id)?;
                if let Some(other) = synced(scene) {
                    self.write_cutscene_exits(other, setup, cutscene, id)?;
                }
            }
            for a in e.attrs(exit, "ExitAddress") {
                self.rom.write_u16(num(a.get("Address")) as u32, id)?;
            }
        } else if actor_params.is_empty() {
            return Err(BuildError::Data(format!("Failed to make any changes for {exit} leading to {spawn}.")));
        }
        if actor_params.is_empty() {
            return Ok(());
        }
        let s = e.attr(spawn, "SpawnActorParams").ok_or_else(|| BuildError::Data(format!("{spawn} has no actor params")))?;
        let opt = |k: &str| s.get(k).as_u64().map(|v| v as u16);
        let (actor, param, mask) = (num(s.get("ActorId")) as u16, num(s.get("Param")) as u16, num(s.get("ParamMask")) as u16);
        let rot = [opt("RotXParam"), opt("RotYParam"), opt("RotZParam")];
        for a in actor_params {
            let (scene, setup, room, index) = (num(a.get("SceneId")) as u8, num(a.get("SetupIndex")) as usize, num(a.get("RoomIndex")) as usize, num(a.get("ActorIndex")) as usize);
            self.write_spawn_actor_params(scene, setup, room, index, actor, param, mask, rot)?;
        }
        Ok(())
    }

    fn scene_index(&self, number: u8) -> Result<usize, BuildError> {
        self.scenes.iter().position(|s| s.number == number).ok_or_else(|| BuildError::Data(format!("no scene {number}")))
    }

    /// `EntranceUtils.WriteSceneExits`.
    fn write_scene_exits(&mut self, scene: u8, index: usize, spawn: u16) -> Result<(), BuildError> {
        let s = self.scene_index(scene)?;
        let file = self.scenes[s].file;
        let lists: Vec<usize> = self.scenes[s].setups.iter().filter_map(|x| x.exit_list_address).collect();
        let data = self.rom.file(file)?;
        for at in lists {
            data[at + index * 2..at + index * 2 + 2].copy_from_slice(&spawn.to_be_bytes());
        }
        Ok(())
    }

    /// `EntranceUtils.WritePolygonTypeExitIndex`.
    fn write_polygon_type_exit(&mut self, scene: u8, polygon_type: usize, new_indices: &[u16], spawn: u16) -> Result<(), BuildError> {
        let s = self.scene_index(scene)?;
        let file = self.scenes[s].file;
        let setups: Vec<(Option<usize>, Option<usize>)> = self.scenes[s].setups.iter().map(|x| (x.polygon_type_definitions_address, x.exit_list_address)).collect();
        let data = self.rom.file(file)?;
        let r16 = |d: &[u8], o: usize| u16::from_be_bytes([d[o], d[o + 1]]);
        for (polygons, exits) in setups {
            let Some(polygons) = polygons else { continue };
            let at = polygons + polygon_type * 8;
            let word = i32::from_be_bytes(data[at..at + 4].try_into().unwrap());
            let exit_index = ((word & 0x1F00) >> 8) - 1;
            if exit_index < 0 {
                return Err(BuildError::Data("Attempt to write exit to non-exit polygon type.".into()));
            }
            let Some(exits) = exits else { continue };
            if r16(data, exits + exit_index as usize * 2) == spawn {
                continue;
            }
            let free = new_indices.iter().copied().find(|&i| {
                let v = r16(data, exits + i as usize * 2);
                v == spawn || v == 0
            });
            let new = free.ok_or_else(|| BuildError::Data("Could not find available exit index.".into()))?;
            let word = (word & !0x1F00) | (((new as i32 + 1) & 0x1F) << 8);
            data[at..at + 4].copy_from_slice(&(word as u32).to_be_bytes());
            data[exits + new as usize * 2..exits + new as usize * 2 + 2].copy_from_slice(&spawn.to_be_bytes());
        }
        Ok(())
    }

    /// `EntranceUtils.WriteCutsceneExits`.
    fn write_cutscene_exits(&mut self, scene: u8, setup: usize, cutscene: usize, spawn: u16) -> Result<(), BuildError> {
        let s = self.scene_index(scene)?;
        let file = self.scenes[s].file;
        let Some(list) = self.scenes[s].setups.get(setup).and_then(|x| x.cutscene_list_address) else { return Ok(()) };
        let at = list + cutscene * 8 + 4;
        self.rom.file(file)?[at..at + 2].copy_from_slice(&spawn.to_be_bytes());
        Ok(())
    }

    /// `EntranceUtils.WriteSceneActorParams`.
    #[allow(clippy::too_many_arguments)]
    fn write_spawn_actor_params(&mut self, scene: u8, setup: usize, room: usize, index: usize, actor: u16, param: u16, mask: u16, rot: [Option<u16>; 3]) -> Result<(), BuildError> {
        let s = self.scene_index(scene)?;
        let r = self.scenes[s].setups.get(setup).and_then(|x| x.rooms.get(room)).cloned().ok_or_else(|| BuildError::Data(format!("scene {scene} has no room {room}")))?;
        let Some(list) = r.actor_list_address else { return Ok(()) };
        let data = self.rom.file(r.file)?;
        let at = list + index * 0x10;
        let r16 = |d: &[u8], o: usize| u16::from_be_bytes([d[o], d[o + 1]]);
        if r16(data, at) & 0x1FFF != actor {
            return Err(BuildError::Data("Attempt to write to incorrect actor entry.".into()));
        }
        let new = (r16(data, at + 0xE) & !mask) | (param & mask);
        data[at + 0xE..at + 0x10].copy_from_slice(&new.to_be_bytes());
        for (k, r) in rot.iter().enumerate() {
            if let Some(v) = r {
                let o = at + 0x8 + k * 2;
                let new = (r16(data, o) & 0x7F) | ((v & 0x1FF) << 7);
                data[o..o + 2].copy_from_slice(&new.to_be_bytes());
            }
        }
        Ok(())
    }
}
