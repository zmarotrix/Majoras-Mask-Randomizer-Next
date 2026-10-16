//! Extra item models: recoloured keys, maps, fairies, tokens and so on, plus
//! the smithy's display models. Port of the old `ExtendedObjects`.

use crate::items::{self, GetItemEntry};
use crate::{be16, be32, BuildError, Builder};

/// Object id and graphic id used to draw an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemGraphic {
    pub object: u16,
    pub graphic: u8,
}

/// Where each extended object starts (`ObjectIndexes`).
#[derive(Debug, Clone, Default)]
pub struct ObjectIndexes {
    pub royal_wallet: Option<u16>,
    pub double_defense: Option<u16>,
    pub magic_power: Option<u16>,
    pub fairies: Option<u16>,
    pub skulltula: Option<u16>,
    pub music_notes: Option<u16>,
    pub rupees: Option<u16>,
    pub milk: Option<u16>,
    pub boss_keys: Option<u16>,
    pub small_keys: Option<u16>,
    pub compasses: Option<u16>,
    pub dungeon_maps: Option<u16>,
    pub notebook_page: Option<u16>,
    pub frogs: Option<u16>,
}

/// A smithy display model (`MiscSmithyModel`).
#[derive(Debug, Clone, Copy)]
pub struct SmithyModel {
    pub old_object: u16,
    pub old_graphic: u8,
    pub new_object: u16,
    pub display_list_offset: u32,
}

impl SmithyModel {
    pub fn to_bytes(self) -> [u8; 12] {
        let mut b = [0u8; 12];
        b[0..2].copy_from_slice(&self.old_object.to_be_bytes());
        b[2] = self.old_graphic;
        b[4..6].copy_from_slice(&self.new_object.to_be_bytes());
        b[8..12].copy_from_slice(&self.display_list_offset.to_be_bytes());
        b
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExtendedObjects {
    /// Start and end of each object in the bundle.
    pub offsets: Vec<(u32, u32)>,
    chunks: Vec<Vec<u8>>,
    pub size: u32,
    pub latest_index: u16,
    pub indexes: ObjectIndexes,
    pub smithy_models: Vec<SmithyModel>,
}

impl ExtendedObjects {
    /// Add data; returns its (start, end) in the bundle. Like the old code,
    /// this pads by `len % 16` bytes (not to a multiple of 16).
    fn append(&mut self, mut data: Vec<u8>) -> (u32, u32) {
        let remaining = data.len() % 0x10;
        if remaining != 0 {
            data.resize(data.len() + remaining, 0);
        }
        let start = self.size;
        self.size += data.len() as u32;
        self.chunks.push(data);
        (start, self.size)
    }

    fn advance(&mut self, amount: u16) -> u16 {
        let i = self.latest_index;
        self.latest_index += amount;
        i
    }

    /// The whole bundle (`ObjectBundle.GetFull`).
    pub fn bundle(&self) -> Vec<u8> {
        self.chunks.concat()
    }

    /// The extended object that draws a get-item entry, if any.
    pub fn resolve(&self, e: &GetItemEntry) -> Option<ItemGraphic> {
        let ix = &self.indexes;
        let g = |object: u16, graphic: u8| Some(ItemGraphic { object, graphic });
        let kind = (e.kind >> 4) as u16;
        match (e.item_gained, e.object) {
            (0xB2, 0x253) if ix.notebook_page.is_some() => g(ix.notebook_page?, 0xC),
            (0xA4, 0xA8) if ix.royal_wallet.is_some() => g(ix.royal_wallet?, 0x22),
            (0xA0, 0xB6) if ix.milk.is_some() => g(ix.milk?, 0x31),
            (0x9F, 0x227) if ix.milk.is_some() => g(ix.milk?, 0x32),
            (0x74, 0x92) if ix.boss_keys.is_some() => g(ix.boss_keys? + kind, e.index),
            (0x78, 0x86) if ix.small_keys.is_some() => g(ix.small_keys? + kind, e.index),
            (0x75, 0x91) if ix.compasses.is_some() => g(ix.compasses? + kind, e.index),
            (0x76, 0xA0) if ix.dungeon_maps.is_some() => g(ix.dungeon_maps? + kind, e.index),
            (0xA6, 0x148) => g(0x148, 0x4B),
            (0x6E, 0x125) if ix.skulltula.is_some() => g(ix.skulltula? + u16::from(e.message == 0x51), e.index),
            (0xA8, 0x13A) if ix.fairies.is_some() => g(ix.fairies? + kind, e.index),
            (0xA7, 0x96) if ix.double_defense.is_some() => g(ix.double_defense?, e.index),
            (0x66..=0x6C | 0x62 | 0x73, 0x8F) if ix.music_notes.is_some() => g(ix.music_notes?, e.index),
            (0xA5, 0xA4) if ix.magic_power.is_some() => g(ix.magic_power?, e.index),
            (0xB1 | 0xB5, 0x13F) if ix.rupees.is_some() => g(ix.rupees?, e.index),
            (0xB4, 0x266) if ix.frogs.is_some() => {
                let graphic = match kind {
                    1 => 0x47,
                    2 => 0x4D,
                    3 => 0x61,
                    4 => 0x62,
                    _ => panic!("unknown frog type"),
                };
                g(ix.frogs?, graphic)
            }
            _ => None,
        }
    }
}

fn put(data: &mut [u8], at: usize, bytes: &[u8]) {
    data[at..at + bytes.len()].copy_from_slice(bytes);
}

const OBJECT_TABLE: u32 = 0xC5_8C80;
const CODE_FILE: usize = 31;

impl Builder<'_> {
    fn clone_file(&mut self, index: usize) -> Result<Vec<u8>, BuildError> {
        Ok(self.rom.file(index)?.clone())
    }

    /// Replace an object file (`ObjUtils.InsertObj`).
    pub(crate) fn insert_object(&mut self, obj: &[u8], replace: u32) -> Result<(), BuildError> {
        let f = self.rom.index_of(OBJECT_TABLE).ok_or(BuildError::Data("no object table".into()))?;
        self.rom.open(f)?;
        let base = (OBJECT_TABLE - self.rom.files[f].addr) as usize + replace as usize * 8;
        let replace_addr = be32(&self.rom.files[f].data, base);
        let Some(objf) = self.rom.files.iter().position(|x| x.addr == replace_addr) else { return Ok(()) };
        let size = (self.rom.files[objf].end - self.rom.files[objf].addr) as usize;
        if obj.len() > size {
            let addr = self.rom.files.last().unwrap().end;
            let end = addr + obj.len() as u32;
            self.rom.files.push(mmr_rom::RomFile {
                addr,
                end,
                compressed: true,
                decompressed: true,
                data: obj.to_vec(),
                cmp_addr: Some(0),
                ..Default::default()
            });
            let old = &mut self.rom.files[objf];
            old.cmp_addr = None;
            old.cmp_end = u32::MAX;
            old.data = Vec::new();
            old.compressed = false;
            let table = &mut self.rom.files[f].data;
            table[base..base + 4].copy_from_slice(&addr.to_be_bytes());
            table[base + 4..base + 8].copy_from_slice(&end.to_be_bytes());
        } else {
            let file = &mut self.rom.files[objf];
            file.data = obj.to_vec();
            file.decompressed = true;
        }
        Ok(())
    }

    /// Build the extended objects (`ExtendedObjects.Create`).
    pub(crate) fn create_extended_objects(&mut self, smithy1: &str, smithy2: &str, fairies: bool, skulltulas: bool, progressive: bool) -> Result<(), BuildError> {
        let mut x = ExtendedObjects { latest_index: 0x283, ..Default::default() };

        // Notebook page: the Bombers' Notebook without its cover label.
        let mut data = self.clone_file(1066)?;
        put(&mut data, 0xB00, &[0u8; 28 * 4]);
        let o = x.append(data);
        x.offsets.push(o);
        x.indexes.notebook_page = Some(x.advance(1));

        // Royal Wallet.
        let mut data = self.clone_file(739)?;
        for (at, rgb) in [
            (0x177C, [0xFF, 0xFF, 0xFF]),
            (0x1784, [0xD0, 0xB0, 0xFF]),
            (0x17FC, [0xA0, 0x40, 0xFF]),
            (0x1804, [0x50, 0x00, 0xC0]),
            (0x181C, [0x80, 0x00, 0xA0]),
            (0x1824, [0x20, 0x20, 0x20]),
            (0x183C, [0xA0, 0x40, 0xFF]),
            (0x1844, [0xFF, 0xC0, 0xFF]),
        ] {
            put(&mut data, at, &rgb);
        }
        let o = x.append(data);
        x.offsets.push(o);
        x.indexes.royal_wallet = Some(x.advance(1));

        // Double Defense.
        let mut data = self.clone_file(726)?;
        put(&mut data, 0x1294, &[0xFF, 0xCF, 0x0F]);
        put(&mut data, 0x12B4, &[0xFF, 0x46, 0x32]);
        put(&mut data, 0x12A8, &[0xFC, 0x17, 0x3C, 0x60, 0x15, 0x0C, 0x93, 0x7F]);
        put(&mut data, 0x1474, &[0xFF, 0xFF, 0xFF]);
        put(&mut data, 0x1494, &[0xFF, 0xFF, 0xFF]);
        let o = x.append(data);
        x.offsets.push(o);
        x.indexes.double_defense = Some(x.advance(1));

        // Magic Power.
        let mut data = self.clone_file(736)?;
        for (at, rgb) in [
            (0x59C, [0xFF, 0xBD, 0x00]),
            (0x654, [0x64, 0xFA, 0x64]),
            (0x7BC, [0x19, 0x7D, 0x32]),
            (0xEFC, [0xFF, 0xBD, 0x00]),
            (0xFB4, [0x64, 0xFA, 0x64]),
            (0x119C, [0x19, 0x7D, 0x32]),
        ] {
            put(&mut data, at, &rgb);
        }
        let o = x.append(data);
        x.offsets.push(o);
        x.indexes.magic_power = Some(x.advance(1));

        // Music notes.
        let mut data = self.clone_file(721)?;
        put(&mut data, 0xAA4, &[0xFF, 0xFF, 0xFF]);
        put(&mut data, 0xAB4, &[0xFF, 0x32, 0x00]);
        let o = x.append(data);
        x.offsets.push(o);
        x.indexes.music_notes = Some(x.advance(1));

        // Rupees.
        let mut data = self.clone_file(825)?;
        for (at, rgb) in [
            (0x4AC + 0x20 * 2, [0xFF, 0x84, 0x55]),
            (0x4B4 + 0x20 * 2, [0x78, 0x00, 0x21]),
            (0x4AC + 0xC0 + 0x20 * 2, [0xFF, 0xE4, 0xC6]),
            (0x4B4 + 0xC0 + 0x20 * 2, [0xCC, 0x00, 0x32]),
            (0x4AC + 0x20 * 4, [0, 0, 0]),
            (0x4B4 + 0x20 * 4, [0, 0, 0]),
            (0x4AC + 0xC0 + 0x20 * 4, [0, 0, 0]),
            (0x4B4 + 0xC0 + 0x20 * 4, [0xFF, 0xFF, 0xFF]),
        ] {
            put(&mut data, at, &rgb);
        }
        let o = x.append(data);
        x.offsets.push(o);
        x.indexes.rupees = Some(x.advance(1));

        // Milk.
        let mut data = self.clone_file(752)?;
        for (at, rgb) in [
            (0x1270 + 0xC, [0xFF, 0xFF, 0xFF]),
            (0x1270 + 0x14, [0x64, 0x64, 0x64]),
            (0x12D0 + 0xC, [0xFF, 0xFF, 0xFF]),
            (0x12D0 + 0x14, [0xFF, 0xFF, 0xFF]),
            (0x1330 + 0xC, [0, 0x20, 0xFF]),
            (0x1330 + 0x14, [0, 0x20, 0xFF]),
            (0x1270 + 0x20 + 0xC, [0xFF, 0xFF, 0xFF]),
            (0x1270 + 0x20 + 0x14, [0x64, 0x64, 0x64]),
            (0x12D0 + 0x20 + 0xC, [0xFF, 0xFF, 0xFF]),
            (0x12D0 + 0x20 + 0x14, [0xFF, 0xFF, 0xFF]),
            (0x1330 + 0x58 + 0xC, [0x6E, 0x46, 0x00]),
            (0x1330 + 0x58 + 0x14, [0x69, 0x0, 0x50]),
        ] {
            put(&mut data, at, &rgb);
        }
        put(&mut data, 0x1698, &[0xDF]);
        let o = x.append(data);
        x.offsets.push(o);
        x.indexes.milk = Some(x.advance(1));

        // Boss keys, small keys, compasses, maps: one per temple.
        let recolor = |b: &mut Self, x: &mut ExtendedObjects, file: usize, at: (usize, usize), colors: [([u8; 3], [u8; 3]); 4]| -> Result<(), BuildError> {
            for (first, second) in colors {
                let mut data = b.clone_file(file)?;
                put(&mut data, at.0, &first);
                put(&mut data, at.1, &second);
                let o = x.append(data);
                x.offsets.push(o);
            }
            Ok(())
        };
        // Boss keys take (env, prim) but write prim first.
        recolor(self, &mut x, 724, (0xF24, 0xF2C), [
            ([0xFF, 0xAA, 0xFF], [0xFF, 0x00, 0x64]),
            ([0xAA, 0xFF, 0xAA], [0x00, 0xFF, 0x32]),
            ([0xAA, 0xFF, 0xFF], [0x00, 0x64, 0xFF]),
            ([0xFF, 0xFF, 0xAA], [0xFF, 0xFF, 0x00]),
        ])?;
        x.indexes.boss_keys = Some(x.advance(4));
        recolor(self, &mut x, 716, (0x81C, 0x824), [
            ([0xFF, 0xAA, 0xFF], [0x73, 0x00, 0x73]),
            ([0xAA, 0xFF, 0xAA], [0x00, 0x73, 0x00]),
            ([0xAA, 0xFF, 0xFF], [0x00, 0x73, 0x73]),
            ([0xFF, 0xFF, 0x00], [0x73, 0x73, 0x00]),
        ])?;
        x.indexes.small_keys = Some(x.advance(4));
        let navigation = [
            ([0x96, 0x32, 0x96], [0x28, 0x14, 0x28]),
            ([0x32, 0x96, 0x32], [0x14, 0x28, 0x14]),
            ([0x32, 0x96, 0x96], [0x14, 0x28, 0x28]),
            ([0x96, 0x96, 0x00], [0x28, 0x28, 0x00]),
        ];
        recolor(self, &mut x, 723, (0x97C, 0x984), navigation)?;
        x.indexes.compasses = Some(x.advance(4));
        recolor(self, &mut x, 733, (0x3DC, 0x3E4), navigation)?;
        x.indexes.dungeon_maps = Some(x.advance(4));

        // Frogs, and get-item draw entries for them.
        let o = x.append(crate::resources::models("gi_frog").unwrap().to_vec());
        x.offsets.push(o);
        for (at, v) in [
            (0x116088, 0x800E_F054u32),
            (0x11608C, 0x0600_0010),
            (0x116160, 0x800E_F054),
            (0x116164, 0x0600_0020),
            (0x116430, 0x800E_F054),
            (0x116434, 0x0600_0030),
            (0x116438, 0),
            (0x116454, 0x800E_F054),
            (0x116458, 0x0600_0040),
            (0x11645C, 0),
        ] {
            self.rom.write_u32(0xB3C000 + at, v)?;
        }
        x.indexes.frogs = Some(x.advance(1));

        // The spin attack energy model goes into the Kokiri Sword object.
        self.insert_object(crate::resources::models("gi_spinattack").unwrap(), 0x148)?;

        if skulltulas {
            for env in [[0x00, 0xFF, 0x00], [0x00, 0x00, 0xFF]] {
                let mut data = self.clone_file(808)?;
                put(&mut data, 0x454, &[0xAA, 0xFF, 0xFF]);
                put(&mut data, 0x45C, &env);
                let o = x.append(data);
                x.offsets.push(o);
            }
            x.indexes.skulltula = Some(x.advance(2));
        }
        if fairies {
            for (outer, inner_prim, inner_env) in [
                ([0xFF, 0xA5, 0x00], [0xFF, 0xFF, 0xDC], [0xFF, 0x80, 0x00]),
                ([0xFF, 0x69, 0xB4], [0xFF, 0xDC, 0xFF], [0xFF, 0x00, 0x64]),
                ([0x00, 0xC0, 0x00], [0xDC, 0xFF, 0xFF], [0x00, 0xFF, 0x32]),
                ([0x18, 0x74, 0xCD], [0xDC, 0xFF, 0xFF], [0x00, 0x64, 0xFF]),
                ([0xFF, 0xFF, 0x00], [0xFF, 0xFF, 0xDC], [0xFF, 0xFF, 0x00]),
            ] {
                let mut data = self.clone_file(823)?;
                put(&mut data, 0xBEC, &outer);
                put(&mut data, 0xF04, &inner_prim);
                put(&mut data, 0xF0C, &inner_env);
                let o = x.append(data);
                x.offsets.push(o);
            }
            x.indexes.fairies = Some(x.advance(5));
        }

        // Smithy display models: a heart, then each smithy item (with its
        // other progressive levels, and refills for bottles).
        self.add_smithy_item(&mut x, SmithySource::Item("RecoveryHeart"))?;
        for item in [smithy1, smithy2] {
            self.add_smithy_item(&mut x, SmithySource::Item(item))?;
            if progressive {
                const PROGRESSIVE: [&[&str]; 5] = [
                    &["StartingSword", "UpgradeRazorSword", "UpgradeGildedSword"],
                    &["FairyMagic", "FairyDoubleMagic"],
                    &["UpgradeAdultWallet", "UpgradeGiantWallet", "UpgradeRoyalWallet"],
                    &["ItemBombBag", "UpgradeBigBombBag", "UpgradeBiggestBombBag"],
                    &["ItemBow", "UpgradeBigQuiver", "UpgradeBiggestQuiver"],
                ];
                if let Some(list) = PROGRESSIVE.iter().find(|l| l.contains(&item)) {
                    for other in list.iter().filter(|o| **o != item) {
                        self.add_smithy_item(&mut x, SmithySource::Item(other))?;
                    }
                }
            }
            let additional = match item {
                "ItemBottleMadameAroma" => Some(0x91),
                "ItemBottleAliens" => Some(0x92),
                "ItemBottleWitch" => Some(0x5B),
                "ItemBottleGoronRace" => Some(0x93),
                _ => None,
            };
            if let Some(gi) = additional {
                self.add_smithy_item(&mut x, SmithySource::GetItem(gi))?;
            }
        }
        self.extended = Some(x);
        Ok(())
    }

    fn add_smithy_item(&mut self, x: &mut ExtendedObjects, source: SmithySource) -> Result<(), BuildError> {
        let graphic = match source {
            SmithySource::Item(item) => match items::get_item_index(item) {
                Some(gi) => self.smithy_graphic(x, gi),
                None => {
                    let e = items::exclusive_entry(item).ok_or_else(|| BuildError::Data(format!("{item} has no model")))?;
                    ItemGraphic { object: e.object, graphic: e.index }
                }
            },
            SmithySource::GetItem(gi) => self.smithy_graphic(x, gi),
        };
        let (offsets, model) = self.smithy_model(x, graphic)?;
        x.offsets.push(offsets);
        x.smithy_models.push(model);
        Ok(())
    }

    fn smithy_graphic(&self, x: &ExtendedObjects, gi: u16) -> ItemGraphic {
        let e = self.get_items[&gi];
        x.resolve(&e).unwrap_or(ItemGraphic { object: e.object, graphic: e.index })
    }

    /// Copy an item's model into a smithy display model (`AddSmithyItem`).
    fn smithy_model(&mut self, x: &mut ExtendedObjects, g: ItemGraphic) -> Result<((u32, u32), SmithyModel), BuildError> {
        let object_id = g.object;
        let object_to_load = if object_id == 0 { 0x1CC } else { object_id }; // 0: assume boss remains
        let graphic_id = ((if g.graphic >= 0x80 { 0x100 - g.graphic as u16 } else { g.graphic as u16 }) as u8).wrapping_sub(1);
        let code = self.rom.file(CODE_FILE)?.clone();
        let object_data = if object_to_load < 0x283 {
            let address = be32(&code, 0x11CC80 + object_to_load as usize * 8);
            let file = self.rom.index_of(address).ok_or(BuildError::Data("missing object file".into()))?;
            self.rom.file(file)?.clone()
        } else {
            x.chunks[(object_to_load - 0x283) as usize].clone()
        };
        const DRAW_ITEM_TABLE: usize = 0x1156B0;
        let dl_at = |index: usize| (be32(&code, DRAW_ITEM_TABLE + graphic_id as usize * 0x24 + index * 4) & 0xFF_FFFF) as usize;
        let lists_to_ignore: &[usize] = match graphic_id {
            10 => &[2],
            66 | 79 | 80 | 81 | 83 | 84 => &[3, 4],
            78 | 98 => &[3],
            89 => &[2],
            _ => &[],
        };
        let vertices_to_ignore: &[usize] = if object_id != 0x148 && graphic_id == 74 { &[2] } else { &[] };

        let mut display_list_index = 1usize;
        let mut dl_offset = dl_at(display_list_index);
        let mut vertices: Vec<[u8; 16]> = Vec::new();
        let mut lists: Vec<[u8; 8]> = Vec::new();
        let mut vertex_commands = 0usize;
        let mut ignoring_triangles = false;
        while dl_offset != 0 {
            if !lists_to_ignore.contains(&display_list_index) {
                let mut entry = 0usize;
                let mut stack: Vec<(usize, usize)> = Vec::new();
                loop {
                    let mut cmd: [u8; 8] = object_data[dl_offset + entry..dl_offset + entry + 8].try_into().unwrap();
                    let kind = cmd[0];
                    match kind {
                        0xDE if cmd[4] == 6 => {
                            if cmd[1] == 0 {
                                stack.push((dl_offset, entry));
                            }
                            dl_offset = (be32(&cmd, 4) & 0xFF_FFFF) as usize;
                            entry = 0;
                            continue;
                        }
                        0xDF if !stack.is_empty() => {
                            let (o, e) = stack.pop().unwrap();
                            dl_offset = o;
                            entry = e + 8;
                            continue;
                        }
                        1 => {
                            if vertices_to_ignore.contains(&vertex_commands) {
                                ignoring_triangles = true;
                                vertex_commands += 1;
                            } else {
                                vertex_commands += 1;
                                ignoring_triangles = false;
                                let count = ((be16(&cmd, 1) & 0x0FF0) >> 4) as usize;
                                let from = (be32(&cmd, 4) & 0xFF_FFFF) as usize;
                                let new_offset = vertices.len() * 0x10;
                                for i in 0..count {
                                    vertices.push(object_data[from + i * 0x10..from + i * 0x10 + 0x10].try_into().unwrap());
                                }
                                cmd[4..8].copy_from_slice(&(new_offset as u32 | 0x0600_0000).to_be_bytes());
                                lists.push(cmd);
                            }
                        }
                        5 | 6 if !ignoring_triangles => lists.push(cmd),
                        _ => {}
                    }
                    entry += 8;
                    if kind == 0xDF {
                        break;
                    }
                }
            }
            if display_list_index == 8 {
                break;
            }
            display_list_index += 1;
            dl_offset = dl_at(display_list_index);
        }
        lists.push([0xDF, 0, 0, 0, 0, 0, 0, 0]);

        let smithy = self.rom.file(958)?.clone();
        let mut result: Vec<u8> = vertices.concat();
        let texture_offset = result.len() as u16;
        result.extend_from_slice(&smithy[0xF6B0..0xF6B0 + 0x100]);
        let display_list_entry = result.len() as u16;
        let mut setup = smithy[0xE8F0..0xE8F0 + 0xC0].to_vec();
        setup[0x46..0x48].copy_from_slice(&texture_offset.to_be_bytes());
        setup[0x7E..0x80].copy_from_slice(&texture_offset.to_be_bytes());
        result.extend_from_slice(&setup);
        result.extend(lists.concat());
        if result.len() % 0x10 != 0 {
            result.extend_from_slice(&[0; 8]);
        }
        let model = SmithyModel { old_object: object_id, old_graphic: graphic_id.wrapping_add(1), new_object: x.advance(1), display_list_offset: display_list_entry as u32 };
        Ok((x.append(result), model))
    }
}

enum SmithySource<'a> {
    Item(&'a str),
    GetItem(u16),
}
