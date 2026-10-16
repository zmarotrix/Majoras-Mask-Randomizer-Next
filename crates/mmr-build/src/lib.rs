//! Builds the randomized ROM: a port of the old randomizer's `Builder`.
//!
//! The build runs in stages, in the old `MakeROM` order. Each stage is checked
//! byte for byte against the old builder (see `tests/legacy_parity.rs`), so
//! the port can't silently change what ends up in the game.

pub mod plan;
pub mod prices;
pub mod resources;
pub mod asm;
pub mod asm_config;
pub mod color;
pub mod dotnet_random;
mod enum_files;
pub mod enums;
pub mod extended;
pub mod fairy_icon;
pub mod hash;
pub mod hash_icons;
pub mod hints;
pub mod items;
pub mod layout;
pub mod manual;
pub mod message;
pub mod music;
mod music_files;
pub mod scene;
pub mod seed;
pub mod spoiler;
pub mod traps;

mod stages;

pub use plan::Plan;
use mmr_rom::{fs::FsError, Rom};

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    Rom(#[from] FsError),
    #[error("{0}")]
    Data(String),
    #[error("building this isn't supported yet: {0}")]
    Unsupported(String),
}

/// Build stages, in order. A build can stop after any of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    PlayerModel,
    Hacks,
    QuickText,
    Speedups,
    Enemies,
    Items,
    Entrances,
    Cutscenes,
    Gimmicks,
    Messages,
    Startup,
    Traps,
    Asm,
    AsmConfig,
    Cosmetics,
    Audio,
}

impl Stage {
    pub const ALL: [Stage; 16] = [
        Stage::PlayerModel,
        Stage::Hacks,
        Stage::QuickText,
        Stage::Speedups,
        Stage::Enemies,
        Stage::Items,
        Stage::Entrances,
        Stage::Cutscenes,
        Stage::Gimmicks,
        Stage::Messages,
        Stage::Startup,
        Stage::Traps,
        Stage::Asm,
        Stage::AsmConfig,
        Stage::Cosmetics,
        Stage::Audio,
    ];

    /// Name used by the legacy harness.
    pub fn name(self) -> &'static str {
        match self {
            Stage::PlayerModel => "player_model",
            Stage::Hacks => "hacks",
            Stage::QuickText => "quick_text",
            Stage::Speedups => "speedups",
            Stage::Enemies => "enemies",
            Stage::Items => "items",
            Stage::Entrances => "entrances",
            Stage::Cutscenes => "cutscenes",
            Stage::Gimmicks => "gimmicks",
            Stage::Messages => "messages",
            Stage::Startup => "startup",
            Stage::Traps => "traps",
            Stage::Asm => "asm",
            Stage::AsmConfig => "asm_config",
            Stage::Cosmetics => "cosmetics",
            Stage::Audio => "audio",
        }
    }
}

/// Build state shared by the stages.
pub struct Builder<'a> {
    pub rom: Rom,
    pub scenes: Vec<scene::Scene>,
    pub plan: &'a Plan,
    /// The game's message table, written back at the messages stage.
    pub messages: message::MessageTable,
    /// New messages for the extended table (written with the asm patch).
    pub extra_messages: Vec<message::MessageEntry>,
    /// Config for the in-game payload, filled in as stages run.
    pub mmr_config: asm_config::MmrConfig,
    pub mask_config: asm_config::MaskConfig,
    /// The get-item table (`RomData.GetItemList`), by get-item index.
    pub get_items: std::collections::HashMap<u16, items::GetItemEntry>,
    /// Bottle catch table entries by index.
    pub bottles: std::collections::HashMap<i64, [u8; 3]>,
    /// Virtual address of the randomizer's get-item table file.
    pub get_item_table: u32,
    pub collectable_table_file: usize,
    pub extended: Option<extended::ExtendedObjects>,
    /// The payload's symbols, once the asm stage has run.
    pub symbols: Option<asm::Symbols>,
    /// Rupees the bank charges per withdrawal.
    pub bank_withdraw_fee: u16,
    /// Trap disguises: get-item index to graphic (`_graphicOverrides`).
    pub graphic_overrides: Vec<(u16, extended::ItemGraphic)>,
    /// How many files the ROM had before building.
    pub original_files: usize,
    /// The build hash, once computed.
    pub hash: Option<Vec<u8>>,
    /// Music packs and loose sequences for random music.
    pub music: Vec<music::MusicFile>,
    /// The payload's per-sequence form mask and track name files.
    pub sequence_mask_file: Option<usize>,
    pub sequence_names_file: Option<usize>,
    pub(crate) sample_patch: Option<stages::audio::SamplePatch>,
    /// What plays where, with random music.
    pub music_placements: Vec<stages::audio::SongPlacement>,
    junk_locations: std::collections::HashSet<String>,
    logically_junk: std::collections::HashSet<String>,
}

/// Extra inputs to a build.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Music packs (`.mmrs`) and loose sequences (`.zseq`) to use with
    /// random music, besides the ones that come with the randomizer.
    pub music: Vec<music::MusicFile>,
    /// Leave out the bundled packs (the old randomizer's own music folder).
    pub no_bundled_music: bool,
    /// Item plando: chosen items at chosen checks.
    pub plando: Vec<mmr_randomizer::plando::PlandoCombo>,
}

/// What a build produces.
#[derive(Debug, Clone)]
pub struct Output {
    pub rom: Vec<u8>,
    /// The build hash (empty if the build stopped before it was computed).
    pub hash: Vec<u8>,
    /// With random music: (slot, song) for every slot.
    pub music: Vec<SongPlacement>,
    /// The file select screen's hash icons.
    pub hash_icons: Vec<hash_icons::HashIcon>,
    /// Inventory icons (32x32 RGBA) of the items in the seed, by item id.
    pub item_icons: std::collections::BTreeMap<String, Vec<u8>>,
}

pub use stages::audio::SongPlacement;

/// The signature MMR Next writes into ROMs (where older versions put the build
/// time, which made identical seeds differ).
pub fn signature() -> Vec<u8> {
    let mut version = format!("MMR {}", env!("CARGO_PKG_VERSION")).into_bytes();
    version.resize(17, b' ');
    [b"MajoraRando\0".as_slice(), &version, b"\0\x01\x02"].concat()
}

/// Build a ROM from a clean (prepared, big-endian) ROM and a plan, running
/// every stage up to and including `until`.
pub fn build(input: &[u8], plan: &Plan, until: Stage) -> Result<Vec<u8>, BuildError> {
    Ok(build_with_hash(input, plan, until)?.0)
}

/// [`build`], also returning the build hash (empty if the build stopped
/// before it was computed).
pub fn build_with_hash(input: &[u8], plan: &Plan, until: Stage) -> Result<(Vec<u8>, Vec<u8>), BuildError> {
    let out = build_with(input, plan, until, &Options::default())?;
    Ok((out.rom, out.hash))
}

/// Build with extra inputs (music packs).
pub fn build_with(input: &[u8], plan: &Plan, until: Stage, options: &Options) -> Result<Output, BuildError> {
    let mut library = if options.no_bundled_music { Vec::new() } else { music::bundled() };
    library.extend(options.music.iter().cloned());
    let mut rom = Rom::read(input)?;
    let scenes = scene::read_scenes(&mut rom)?;
    let messages = message::MessageTable::read_default(&mut rom)?;
    let mut b = Builder {
        rom,
        scenes,
        plan,
        messages,
        extra_messages: Vec::new(),
        mmr_config: Default::default(),
        mask_config: Default::default(),
        get_items: Default::default(),
        bottles: Default::default(),
        get_item_table: 0,
        collectable_table_file: 0,
        extended: None,
        symbols: None,
        bank_withdraw_fee: 4,
        graphic_overrides: Vec::new(),
        original_files: 0,
        hash: None,
        music: music::sorted(library),
        sequence_mask_file: None,
        sequence_names_file: None,
        sample_patch: None,
        music_placements: Vec::new(),
        junk_locations: plan.junk_locations.iter().cloned().collect(),
        logically_junk: plan.logically_junk_items.iter().cloned().collect(),
    };
    b.original_files = b.rom.files.len();
    for stage in Stage::ALL.into_iter().filter(|s| *s <= until) {
        b.run(stage)?;
    }
    let icons = b.hash_icons()?;
    let mut wanted: Vec<String> = plan.items.iter().flat_map(|p| [p.item.clone(), p.id.clone()]).collect();
    wanted.extend(plan.items.iter().filter_map(|p| p.mimic.as_ref().map(|m| m.item.clone())));
    wanted.extend(plan.custom_starting_item_list.iter().cloned());
    wanted.sort();
    wanted.dedup();
    let item_icons = if until >= Stage::Items { b.item_icons(&wanted)? } else { Default::default() };
    let patch = b.sample_patch.take();
    let rom = b.rom.build_with(&signature(), |out, files| {
        if let Some(p) = &patch {
            stages::audio::update_bank_instrument_pointers(out, files, p);
        }
    })?;
    Ok(Output { rom, hash: b.hash.unwrap_or_default(), music: b.music_placements, hash_icons: icons, item_icons })
}

impl Builder<'_> {
    fn run(&mut self, stage: Stage) -> Result<(), BuildError> {
        match stage {
            Stage::PlayerModel => self.write_player_model(),
            Stage::Hacks => self.write_hacks(),
            Stage::QuickText => self.write_quick_text(),
            Stage::Speedups => self.write_speedups(),
            Stage::Enemies => self.write_enemies(),
            Stage::Items => self.write_items(),
            Stage::Entrances => self.write_entrances(),
            Stage::Messages => self.write_messages(),
            Stage::Startup => self.write_startup(),
            Stage::Traps => self.write_traps(),
            Stage::Asm => self.write_asm(),
            Stage::AsmConfig => self.write_asm_config(),
            Stage::Cosmetics => self.write_cosmetics(),
            Stage::Audio => self.write_audio(),
            Stage::Cutscenes => self.write_cutscenes(),
            Stage::Gimmicks => {
                self.write_gimmicks()?;
                self.write_misc_hacks()?;
                self.write_moon_child_denial_hack()
            }
        }
    }

    /// Apply a binary patch: a list of (address, length, bytes), ended by 0xFF
    /// (`ResourceUtils.ApplyHack`).
    pub fn apply_hack(&mut self, hack: &[u8]) -> Result<(), BuildError> {
        let mut at = 0usize;
        while at < hack.len() && hack[at] != 0xFF {
            let dest = be32(hack, at);
            let len = be32(hack, at + 4) as usize;
            at += 8;
            self.rom.write(dest, &hack[at..at + len])?;
            at += len;
        }
        Ok(())
    }

    /// Apply a named patch from the old randomizer's resources.
    pub fn apply_mod(&mut self, group: &str, name: &str) -> Result<(), BuildError> {
        let hack = match group {
            "mods" => resources::mods(name),
            "models" => resources::models(name),
            "addresses" => resources::addresses(name),
            "asm" => resources::asm(name),
            _ => None,
        }
        .ok_or_else(|| BuildError::Data(format!("missing resource {group}/{name}")))?;
        self.apply_hack(hack)
    }
}

pub(crate) fn be16(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}

pub(crate) fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes(b[o..o + 4].try_into().unwrap())
}
