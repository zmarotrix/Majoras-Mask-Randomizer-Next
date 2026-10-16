//! Scenes and rooms: where actors, exits and cutscenes live. Port of the old
//! `SceneUtils`.

use crate::{be16, be32, BuildError};
use mmr_rom::Rom;

const SCENE_TABLE: u32 = 0xC5_A1E0;
const SCENE_FLAG_MASKS: u32 = 0xC5_C500;

#[derive(Debug, Clone, Default)]
pub struct Actor {
    /// Upper 4 bits of the actor number (spawn flags).
    pub m: u16,
    /// Actor number.
    pub n: u16,
    pub pos: [i16; 3],
    pub rot: [i16; 3],
    pub variable: u16,
}

#[derive(Debug, Clone, Default)]
pub struct Map {
    pub file: usize,
    pub header: usize,
    pub actor_addr: usize,
    pub obj_addr: usize,
    pub objects: Vec<u16>,
    pub actors: Vec<Actor>,
}

#[derive(Debug, Clone, Default)]
pub struct Room {
    pub file: usize,
    pub actor_list_address: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ActorCutscene {
    pub unknown00: u16,
    /// -1: plays constantly.
    pub length: i16,
    pub camera_index: i16,
    pub cutscene_index: i16,
    pub additional_actor_cutscene_index: i16,
    pub sound: u8,
    pub unknown0b: u8,
    pub hud_fade: i16,
    pub return_camera_type: u8,
    pub letterbox: u8,
}

impl ActorCutscene {
    pub fn read(d: &[u8]) -> Self {
        let s16 = |o| be16(d, o) as i16;
        ActorCutscene {
            unknown00: be16(d, 0),
            length: s16(2),
            camera_index: s16(4),
            cutscene_index: s16(6),
            additional_actor_cutscene_index: s16(8),
            sound: d[0xA],
            unknown0b: d[0xB],
            hud_fade: s16(0xC),
            return_camera_type: d[0xE],
            letterbox: d[0xF],
        }
    }
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut r = [0u8; 16];
        r[0..2].copy_from_slice(&self.unknown00.to_be_bytes());
        r[2..4].copy_from_slice(&self.length.to_be_bytes());
        r[4..6].copy_from_slice(&self.camera_index.to_be_bytes());
        r[6..8].copy_from_slice(&self.cutscene_index.to_be_bytes());
        r[8..10].copy_from_slice(&self.additional_actor_cutscene_index.to_be_bytes());
        r[0xA] = self.sound;
        r[0xB] = self.unknown0b;
        r[0xC..0xE].copy_from_slice(&self.hud_fade.to_be_bytes());
        r[0xE] = self.return_camera_type;
        r[0xF] = self.letterbox;
        r
    }
}

#[derive(Debug, Clone, Default)]
pub struct SceneSetup {
    pub rooms: Vec<Room>,
    pub collision_header_address: Option<usize>,
    pub polygon_type_definitions_address: Option<usize>,
    pub exit_list_address: Option<usize>,
    pub cutscene_list_address: Option<usize>,
    pub actor_cutscene_list_address: Option<usize>,
    pub actor_cutscenes: Option<Vec<ActorCutscene>>,
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub file: usize,
    pub number: u8,
    pub maps: Vec<Map>,
    pub setups: Vec<SceneSetup>,
}

fn file_at(rom: &Rom, addr: u32) -> Result<usize, BuildError> {
    rom.index_of(addr).ok_or(BuildError::Rom(mmr_rom::fs::FsError::NoFile(addr)))
}

/// Read the scene table, every scene's rooms and setups (`ReadSceneTable`
/// then `GetMaps`). Like the old code, this opens every scene and room file.
pub fn read_scenes(rom: &mut Rom) -> Result<Vec<Scene>, BuildError> {
    let mut scenes = Vec::new();
    let f = rom.index_of(SCENE_TABLE).ok_or(BuildError::Rom(mmr_rom::fs::FsError::NoFile(SCENE_TABLE)))?;
    rom.open(f)?;
    let base = (SCENE_TABLE - rom.files[f].addr) as usize;
    let mut i = 0usize;
    loop {
        let addr = be32(&rom.files[f].data, base + i);
        if addr > 0x400_0000 {
            break;
        }
        if addr != 0 {
            scenes.push(Scene { file: file_at(rom, addr)?, number: (i >> 4) as u8, ..Default::default() });
        }
        i += 16;
    }

    for scene in &mut scenes {
        let f = scene.file;
        rom.open(f)?;
        let mut j = 0usize;
        loop {
            let data = &rom.files[f].data;
            let cmd = data[j];
            if cmd == 0x04 {
                let count = data[j + 1];
                let mut maps_addr = (be32(data, j + 4) & 0xFF_FFFF) as usize;
                for _ in 0..count {
                    let file = file_at(rom, be32(&rom.files[f].data, maps_addr))?;
                    scene.maps.push(Map { file, ..Default::default() });
                    maps_addr += 8;
                }
                break;
            }
            if cmd == 0x14 {
                break;
            }
            j += 8;
        }
        check_scene_headers(rom, f, 0, scene)?;
        // Unused setup in East Clock Town; the old code leaves it alone.
        if scene.number == 108 {
            scene.setups.remove(2);
        }
    }
    Ok(scenes)
}

fn check_scene_headers(rom: &mut Rom, f: usize, header: usize, scene: &mut Scene) -> Result<(), BuildError> {
    let mut j = header;
    let mut setups_addr: Option<usize> = None;
    let mut next_lowest: Option<usize> = None;
    let mut setup = SceneSetup::default();
    let setup_index = scene.setups.len();
    scene.setups.push(SceneSetup::default());
    loop {
        let data = &rom.files[f].data;
        let cmd = data[j];
        let ptr = (be32(data, j + 4) & 0xFF_FFFF) as usize;
        match cmd {
            0x03 => {
                setup.collision_header_address = Some(ptr);
                setup.polygon_type_definitions_address = Some((be32(data, ptr + 0x1C) & 0xFF_FFFF) as usize);
            }
            0x04 => {
                let count = data[j + 1];
                let mut room_addr = ptr;
                for _ in 0..count {
                    let file = file_at(rom, be32(&rom.files[f].data, room_addr))?;
                    room_addr += 8;
                    let mut room = Room { file, actor_list_address: None };
                    check_room_headers(rom, file, setup_index, &mut room)?;
                    setup.rooms.push(room);
                }
            }
            0x13 => setup.exit_list_address = Some(ptr),
            0x17 => setup.cutscene_list_address = Some(ptr),
            0x18 => setups_addr = Some(ptr),
            0x1B => {
                let count = data[j + 1] as usize;
                setup.actor_cutscene_list_address = Some(ptr);
                setup.actor_cutscenes = Some((0..count).map(|i| ActorCutscene::read(&data[ptr + i * 0x10..ptr + i * 0x10 + 0x10])).collect());
            }
            0x14 => break,
            _ => {
                if data[j + 4] == 0x02 {
                    if let Some(s) = setups_addr {
                        if ptr > s && next_lowest.is_none_or(|n| ptr < n) {
                            next_lowest = Some(ptr);
                        }
                    }
                }
            }
        }
        j += 8;
    }
    scene.setups[setup_index] = setup;
    if let (Some(s), Some(_)) = (setups_addr, next_lowest) {
        let mut j = s;
        while rom.files[f].data[j] == 0x02 {
            let p = (be32(&rom.files[f].data, j) & 0xFF_FFFF) as usize;
            check_scene_headers(rom, f, p, scene)?;
            j += 4;
        }
    }
    Ok(())
}

fn check_room_headers(rom: &mut Rom, f: usize, setup_index: usize, room: &mut Room) -> Result<(), BuildError> {
    rom.open(f)?;
    let data = &rom.files[f].data;
    let mut j = 0usize;
    if setup_index > 0 {
        if data[0] != 0x18 {
            return Err(BuildError::Data(format!("room file {f}: first header is not alternate headers")));
        }
        let headers = (be32(data, 4) & 0xFF_FFFF) as usize;
        j = (be32(data, headers + (setup_index - 1) * 4) & 0xFF_FFFF) as usize;
    }
    loop {
        let cmd = data[j];
        if cmd == 0x01 {
            room.actor_list_address = Some((be32(data, j + 4) & 0xFF_FFFF) as usize);
        } else if cmd == 0x14 {
            break;
        }
        j += 8;
    }
    Ok(())
}

/// Clear the first words of the scene flag masks (`ResetSceneFlagMask`).
pub fn reset_flag_mask(rom: &mut Rom) -> Result<(), BuildError> {
    rom.write_u32(SCENE_FLAG_MASKS, 0)?;
    rom.write_u32(SCENE_FLAG_MASKS + 0xC, 0)?;
    Ok(())
}

/// Mark a get-item index as a one-time item (`UpdateSceneFlagMask`).
pub fn update_flag_mask(rom: &mut Rom, num: u32) -> Result<(), BuildError> {
    let mut offset = (num >> 3) as i64;
    if num >= 0x380 {
        offset += 0x20;
    }
    if num >= 0x400 {
        offset += 0x40;
    }
    let m = offset % 16;
    if m < 4 {
        offset += 8;
    } else if m < 12 {
        offset -= 4;
    }
    let bit = 1u8 << (num & 7);
    let b = rom.bytes_mut(SCENE_FLAG_MASKS + offset as u32, 1)?;
    b[0] |= bit;
    Ok(())
}

/// Add each room's alternate headers as maps of their own, then read every
/// map's actors and objects (`GetMapHeaders` then `GetActors`).
pub fn read_map_actors(rom: &mut Rom, scenes: &mut [Scene]) -> Result<(), BuildError> {
    for scene in scenes.iter_mut() {
        let originals = scene.maps.len();
        for m in 0..originals {
            let f = scene.maps[m].file;
            rom.open(f)?;
            let data = &rom.files[f].data;
            let mut k = 0usize;
            let mut setups: Option<usize> = None;
            let mut next_lowest: Option<usize> = None;
            loop {
                let cmd = data[k];
                if cmd == 0x18 {
                    setups = Some((be32(data, k + 4) & 0xFF_FFFF) as usize);
                } else if cmd == 0x14 {
                    break;
                } else if data[k + 4] == 0x03 {
                    let p = (be32(data, k + 4) & 0xFF_FFFF) as usize;
                    if next_lowest.is_none_or(|n| p < n) && setups.is_some_and(|s| p > s) {
                        next_lowest = Some(p);
                    }
                }
                k += 8;
            }
            let (Some(s), Some(n)) = (setups, next_lowest) else { continue };
            let mut k = s;
            while k < n && data[k] == 0x03 {
                scene.maps.push(Map { file: f, header: (be32(data, k) & 0xFF_FFFF) as usize, ..Default::default() });
                k += 4;
            }
        }
        for map in scene.maps.iter_mut() {
            rom.open(map.file)?;
            let data = &rom.files[map.file].data;
            let mut k = map.header;
            loop {
                let cmd = data[k];
                if cmd == 0x01 {
                    let count = data[k + 1] as usize;
                    let at = (be32(data, k + 4) & 0xFF_FFFF) as usize;
                    map.actor_addr = at;
                    map.actors = (0..count)
                        .map(|i| {
                            let a = at + i * 16;
                            let s = |o: usize| be16(data, a + o) as i16;
                            let id = be16(data, a);
                            Actor { m: id & 0xF000, n: id & 0x0FFF, pos: [s(2), s(4), s(6)], rot: [s(8), s(10), s(12)], variable: be16(data, a + 14) }
                        })
                        .collect();
                }
                if cmd == 0x0B {
                    let count = data[k + 1] as usize;
                    let at = (be32(data, k + 4) & 0xFF_FFFF) as usize;
                    map.obj_addr = at;
                    map.objects = (0..count).map(|i| be16(data, at + i * 2)).collect();
                }
                if cmd == 0x14 {
                    break;
                }
                k += 8;
            }
        }
    }
    Ok(())
}

/// Write a scene's maps' actors and objects back (`UpdateScene`).
pub fn write_maps(rom: &mut Rom, scene: &Scene) -> Result<(), BuildError> {
    for map in &scene.maps {
        let data = rom.file(map.file)?;
        for (i, a) in map.actors.iter().enumerate() {
            let at = map.actor_addr + i * 16;
            let words = [a.m | a.n, a.pos[0] as u16, a.pos[1] as u16, a.pos[2] as u16, a.rot[0] as u16, a.rot[1] as u16, a.rot[2] as u16, a.variable];
            for (w, v) in words.iter().enumerate() {
                data[at + w * 2..at + w * 2 + 2].copy_from_slice(&v.to_be_bytes());
            }
        }
        for (i, o) in map.objects.iter().enumerate() {
            data[map.obj_addr + i * 2..map.obj_addr + i * 2 + 2].copy_from_slice(&o.to_be_bytes());
        }
    }
    Ok(())
}
