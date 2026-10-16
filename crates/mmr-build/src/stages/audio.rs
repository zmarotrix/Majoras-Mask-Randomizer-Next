//! Music (`WriteAudioSeq` with `SequenceUtils`): shuffles the music when
//! asked, then rebuilds the sequence file and instrument banks.

use crate::dotnet_random::DotNetRandom;
use crate::music::{self, Bank, BankRef, MusicFile, Song};
use crate::stages::cosmetics::NEW_INSTRUMENT_SET_ADDRESS;
use crate::{be32, BuildError, Builder};
use std::cell::RefCell;
use std::rc::Rc;

const INST_SET_MAP: u32 = 0xC77A60;
const SEQ_TABLE: u32 = 0xC77B80;
const AUDIOBANK: u32 = 0x020700;
const AUDIOSEQ_FILE: usize = 4;
/// The first bank slot free for custom banks (`ResetFreeBankIndex`).
const FIRST_FREE_BANK: i32 = 0x29;
/// Sequence and BGM/combat buffer sizes (`MAX_*_BUDGET`).
const BUDGET: i32 = 0x6000;
/// Per-sequence form mask and name sizes in the payload's music files.
pub(crate) const SEQUENCE_DATA_SIZE: usize = 0x30;
pub(crate) const SEQUENCE_NAME_MAX_SIZE: usize = 0x20;
pub(crate) const NUM_SEQUENCES: usize = 0x80;

/// A slot the game plays music from (a vanilla `SequenceInfo` target).
#[derive(Debug, Clone)]
struct Slot {
    name: String,
    categories: Vec<i32>,
    replaces: i32,
    previous_slot: i32,
}

/// What the later ROM layout needs to point banks at custom samples
/// (`UpdateBankInstrumentPointers`).
#[derive(Debug, Default)]
pub(crate) struct SamplePatch {
    pub samples_file: usize,
    /// Per bank, in table order: its length, and its samples' (marker, address
    /// in the samples file).
    pub banks: Vec<(usize, Vec<(u32, u32)>)>,
}

/// One song placed in a slot, for the spoiler log.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SongPlacement {
    pub slot: String,
    pub song: String,
}

/// Music shuffle state (`RomData.SequenceList`, `TargetSequences`, ...).
struct Music {
    songs: Vec<Song>,
    slots: Vec<Slot>,
    pointerized: Vec<Slot>,
    banks: Vec<BankRef>,
    free_bank: i32,
    bgm_budget: i32,
    combat_budget: i32,
    unassigned: Vec<usize>,
    luck: f64,
    log: Vec<String>,
}

fn round16(v: usize) -> i32 {
    ((v + 0xF) & !0xF) as i32
}

fn intersects(a: &[i32], b: &[i32]) -> bool {
    a.iter().any(|x| b.contains(x))
}

fn categories_text(c: &[i32]) -> String {
    c.iter().map(|x| format!("0x{x:02X}")).collect::<Vec<_>>().join(", ")
}

/// .NET's `OrderBy(x => random.Next())`: one key per item in order, then a
/// stable sort.
fn shuffle<T>(items: Vec<T>, random: &mut DotNetRandom) -> Vec<T> {
    let mut keyed: Vec<(i32, T)> = items.into_iter().map(|t| (random.next(), t)).collect();
    keyed.sort_by_key(|(k, _)| *k);
    keyed.into_iter().map(|(_, t)| t).collect()
}

/// `(decimal)(x) < limit`: .NET rounds the double to 15 significant digits first.
fn decimal_less(x: f64, limit: f64) -> bool {
    let rounded: f64 = format!("{x:.14e}").parse().unwrap_or(x);
    rounded < limit
}

impl Builder<'_> {
    pub(crate) fn write_audio(&mut self) -> Result<(), BuildError> {
        let choice = self.cosmetic("Music").as_str().unwrap_or("Default").to_string();
        if choice == "None" {
            return Ok(());
        }
        let random_music = choice == "Random";
        let mut m = self.read_sequence_info(random_music)?;
        m.banks = self.read_instrument_sets()?;
        if random_music {
            let h = self.hash.clone().unwrap_or_default();
            let mut random = DotNetRandom::new(i32::from_le_bytes([h[0], h[1], h[2], h[3]]));
            self.pointerize_sequence_slots(&mut m);
            self.bgm_shuffle(&mut m, &mut random)?;
            self.apply_mod("mods", "remove_morning_music")?;
        }
        self.apply_mod("mods", "fix_music")?;
        self.rebuild_audio_seq(&mut m)?;
        self.write_new_sound_samples(&m);
        self.rebuild_audio_bank(&m)?;
        Ok(())
    }

    /// `ReadSequenceInfo`: the vanilla slots and songs from `SEQS.txt`, the
    /// Song of Time track, then (for random music) loose sequences and packs.
    fn read_sequence_info(&self, with_packs: bool) -> Result<Music, BuildError> {
        let text = include_str!("../../../../data/rom/music/SEQS.txt");
        let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
        let hex = |s: &str| music::hex(s).map_err(BuildError::Data);
        let mut songs: Vec<Song> = Vec::new();
        let mut slots: Vec<Slot> = Vec::new();
        let files: Vec<MusicFile> = if with_packs { self.music.clone() } else { Vec::new() };
        let mut i = 0;
        while i + 2 < lines.len() {
            let name = lines[i];
            let categories = lines[i + 1].split(',').map(&hex).collect::<Result<Vec<_>, _>>()?;
            let instrument = hex(lines[i + 2])?;
            let mut song = Song::new(name);
            song.categories = categories.clone();
            song.instrument = instrument;
            if name.starts_with("mm-") {
                let slot = hex(lines.get(i + 3).copied().unwrap_or(""))?;
                song.mm_seq = slot;
                if lines.get(i + 4) == Some(&"no-recycle") {
                    song.name = "drop".into();
                    i += 1;
                }
                i += 4;
                if slots.iter().any(|s| s.name == name) {
                    continue;
                }
                slots.push(Slot { name: name.into(), categories, replaces: slot, previous_slot: -1 });
            } else {
                // Other games' sequences, if the file is in the music folder.
                i += 3;
                match files.iter().find(|f| f.name == name) {
                    Some(f) => song.file = Some(f.data.clone()),
                    None => continue,
                }
            }
            if song.mm_seq != 0x18 && song.name != "drop" {
                songs.push(song);
            }
        }
        // The randomizer's own Song of Time track, in an unused slot.
        let mut sot = Song::new("mmr_f_sot");
        sot.categories = vec![8];
        sot.instrument = 3;
        sot.replaces = 0x75;
        songs.push(sot);
        for f in files.iter().filter(|f| f.name.to_ascii_lowercase().ends_with(".zseq")) {
            if let Some(s) = music::read_zseq(f).map_err(BuildError::Data)? {
                songs.push(s);
            }
        }
        for f in files.iter().filter(|f| f.name.to_ascii_lowercase().ends_with(".mmrs")) {
            // Broken packs are skipped, as before.
            if let Ok(s) = music::read_pack(f) {
                songs.push(s);
            }
        }
        let luck = self.cosmetic("MusicLuckRollChance").as_f64().unwrap_or(3.33);
        Ok(Music {
            songs,
            slots,
            pointerized: Vec::new(),
            banks: Vec::new(),
            free_bank: FIRST_FREE_BANK,
            bgm_budget: BUDGET,
            combat_budget: BUDGET,
            unassigned: Vec::new(),
            luck,
            log: Vec::new(),
        })
    }

    /// Every instrument bank (`ReadInstrumentSetList`).
    fn read_instrument_sets(&mut self) -> Result<Vec<BankRef>, BuildError> {
        let mut out = Vec::new();
        for index in 0..=0x80u32 {
            let at = NEW_INSTRUMENT_SET_ADDRESS + index * 0x10;
            let offset = ((self.rom.read_u16(at)? as u32) << 16) + self.rom.read_u16(at + 2)? as u32;
            let length = ((self.rom.read_u16(at + 4)? as u32) << 16) + self.rom.read_u16(at + 6)? as u32;
            let mut meta = vec![0u8; 8];
            for (b, m) in meta.iter_mut().enumerate() {
                *m = self.rom.read_u8(at + 8 + b as u32)?;
            }
            let mut data = vec![0u8; length as usize];
            for (b, d) in data.iter_mut().enumerate() {
                *d = self.rom.read_u8(AUDIOBANK + offset + b as u32)?;
            }
            out.push(Rc::new(RefCell::new(Bank { data: Rc::new(data), slot: index as i32, meta, modified: 0, hash: 0, samples: None })));
        }
        Ok(out)
    }

    /// The size a song takes in the sequence file (`GetSequenceSize`).
    fn sequence_size(&mut self, song: &Song) -> Result<i32, BuildError> {
        if let Some(b) = song.binaries.as_ref().and_then(|b| b.first()) {
            return Ok(round16(b.data.len()));
        }
        if song.name.starts_with("mm-") {
            let f = self.rom.index_of(SEQ_TABLE).unwrap();
            self.rom.open(f)?;
            let at = (SEQ_TABLE - self.rom.files[f].addr) as usize + song.mm_seq as usize * 16;
            return Ok(round16(be32(&self.rom.files[f].data, at + 4) as usize));
        }
        match &song.file {
            Some(d) => Ok(round16(d.len())),
            None => Err(BuildError::Data(format!("music: sequence file {} is missing", song.name))),
        }
    }

    /// Point a slot at another slot's music (`ConvertSequenceSlotToPointer`).
    fn pointerize(m: &mut Music, slot: i32, substitute: i32) {
        let target = m.slots.iter().position(|s| s.replaces == slot);
        let sub = m.slots.iter().find(|s| s.replaces == substitute).map(|s| s.replaces);
        if let (Some(t), Some(sub)) = (target, sub) {
            let mut s = m.slots.remove(t);
            s.previous_slot = s.replaces;
            s.replaces = sub;
            m.pointerized.push(s);
        }
    }

    /// Slots rarely or never heard share another slot's music, so there's
    /// enough to go round (`PointerizeSequenceSlots`).
    fn pointerize_sequence_slots(&mut self, m: &mut Music) {
        Self::pointerize(m, 0x29, 0x0B);
        // Shortened cutscenes (detected by a change in Great Bay's cutscene scene).
        if self.rom.files[1472].data[0xD48 + 3] != 0x94 {
            Self::pointerize(m, 0x72, 0x45);
            Self::pointerize(m, 0x2D, 0x3A);
            Self::pointerize(m, 0x70, 0x0B);
            Self::pointerize(m, 0x7B, 0x0D);
            Self::pointerize(m, 0x7D, 0x05);
        }
        // Skull Kid's backstory, if the ocarina isn't randomized.
        if self.rom.files[31].data[0x8A6A0 + 1] == 0xF9 {
            Self::pointerize(m, 0x04, 0x1A);
        }
        if m.slots.len() + 30 > m.songs.len() {
            Self::pointerize(m, 0x76, 0x15);
            Self::pointerize(m, 0x08, 0x09);
            Self::pointerize(m, 0x19, 0x78);
        }
        m.pointerized.push(Slot { name: "mm-introcutscene1".into(), categories: Vec::new(), replaces: 0x76, previous_slot: 0x1E });
    }

    /// `BGMShuffle`.
    fn bgm_shuffle(&mut self, m: &mut Music, random: &mut DotNetRandom) -> Result<(), BuildError> {
        // Shuffle both songs and slots, so variety isn't used up by whichever
        // slots come first.
        let unassigned: Vec<usize> = (0..m.songs.len()).filter(|&i| m.songs[i].replaces == -1).collect();
        m.unassigned = shuffle(unassigned, random);
        m.slots = shuffle(std::mem::take(&mut m.slots), random);
        m.log.push(format!(" Randomizing {} song slots, with {} available songs:", m.slots.len(), m.unassigned.len()));
        m.bgm_budget = BUDGET;
        m.combat_budget = BUDGET;
        self.check_song_test(m)?;
        self.music_plando(m, random)?;
        // Songs marked "songforce" go to the front of the queue.
        let forced: Vec<usize> = (0..m.songs.len()).filter(|&i| m.songs[i].name.contains("songforce")).collect();
        for i in shuffle(forced, random) {
            m.log.push(format!("Forcing song ({}) to top of the song pool", m.songs[i].name));
            m.unassigned.retain(|&u| u != i);
            m.unassigned.insert(0, i);
        }
        for slot in m.slots.clone() {
            if !self.search_for_song(m, &slot, random)? {
                m.log.push(format!("No song fits in [{}] slot, with categories: {}", slot.name, categories_text(&slot.categories)));
                self.backup_song_placement(m, &slot)?;
            }
        }
        self.check_music_budget(m, random)?;
        m.songs.retain(|s| s.replaces != -1);
        // A bigger music buffer.
        self.write_code_u32(0x801D_B9B4, 0x6000)?;
        self.write_code_u32(0x801D_B9B8, 0x6000)?;
        Ok(())
    }

    /// Put a song in a slot, claiming a bank for it if it brings its own
    /// (`AssignSequenceSlot`).
    fn assign(m: &mut Music, slot: &Slot, song: usize, note: &str) -> Result<(), BuildError> {
        let first = match &m.songs[song].binaries {
            Some(b) => Some(b.first().cloned().ok_or_else(|| BuildError::Data(format!("music: {} has no usable sequence", m.songs[song].name)))?),
            None => None,
        };
        if let Some(bank) = first.as_ref().and_then(|b| b.bank.clone()) {
            // (The old duplicate-bank search compared array references, so it
            // never found one: every song with a bank claims a new slot.)
            let index = m.free_bank;
            m.free_bank += 1;
            bank.borrow_mut().slot = index;
            bank.borrow_mut().samples = m.songs[song].samples.clone();
            let slot_bank = m.banks.get_mut(index as usize).ok_or_else(|| BuildError::Data("music: out of instrument banks".into()))?;
            *slot_bank = bank;
            m.songs[song].instrument = index;
            m.log.push(format!(" -- v -- Instrument set number {index:02X} has been claimed -- v --"));
            m.songs[song].binaries = Some(vec![first.unwrap()]);
        }
        m.songs[song].replaces = slot.replaces;
        m.log.push(format!("{:<40} {:>10} -> {}", slot.name, note, m.songs[song].name));
        if let Some(p) = m.unassigned.iter().position(|&u| u == song) {
            m.unassigned.remove(p);
        }
        Ok(())
    }

    /// Whether a song's bank (if it needs one) can still be placed
    /// (`TestIfAvailableBanks`).
    fn banks_available(m: &mut Music, song: usize, random: &mut DotNetRandom) -> bool {
        let Some(list) = m.songs[song].binaries.clone() else { return true };
        if list.is_empty() || list.iter().all(|b| b.bank.is_none()) {
            return true;
        }
        let list = if list.len() > 1 { shuffle(list, random) } else { list };
        let usable = |b: &Rc<music::SeqBinary>| match &b.bank {
            None => true,
            Some(bank) => {
                let bank = bank.borrow();
                let current = m.banks[bank.slot as usize].borrow();
                current.modified == 0 || (bank.hash != 0 && bank.hash == current.hash)
            }
        };
        if list.iter().any(usable) {
            let kept: Vec<_> = list.iter().filter(|b| usable(b)).cloned().collect();
            m.songs[song].binaries = Some(kept);
        } else {
            if m.free_bank > 0x80 {
                m.songs[song].binaries = Some(list);
                return false;
            }
            list[0].bank.as_ref().unwrap().borrow_mut().slot = m.free_bank;
            m.songs[song].binaries = Some(list);
        }
        true
    }

    /// Find a song for a slot (`SearchForValidSongReplacement`).
    fn search_for_song(&mut self, m: &mut Music, slot: &Slot, random: &mut DotNetRandom) -> Result<bool, BuildError> {
        for song in m.unassigned.clone() {
            if !Self::banks_available(m, song, random) {
                continue;
            }
            let max = if slot.replaces == 0x1A { m.combat_budget } else { m.bgm_budget };
            let s = m.songs[song].clone();
            if self.sequence_size(&s)? > max {
                continue;
            }
            if intersects(&s.categories, &slot.categories) {
                Self::assign(m, slot, song, "")?;
                return Ok(true);
            }
            // A small chance of music from another category (but never
            // fanfares as background music or the other way round).
            let (sc, tc) = (&s.categories, &slot.categories);
            if m.unassigned.len() > 30
                && sc.len() > tc.len()
                && m.luck > 0.0
                && decimal_less(random.next_double() * 100.0, m.luck)
                && tc[0] <= 0x16
                && sc[0] <= 0x16
                && (sc[0] & 8) == (tc[0] & 8)
                && sc.contains(&0x10) == tc.contains(&0x10)
                && !sc.contains(&0x16)
            {
                Self::assign(m, slot, song, "LUCK")?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Loosen the rules for a slot nothing fits (`TryBackupSongPlacement`).
    fn backup_song_placement(&mut self, m: &mut Music, slot: &Slot) -> Result<(), BuildError> {
        let first = |s: &Song| s.categories.first().copied().unwrap_or(-1);
        let t = first(&Song { categories: slot.categories.clone(), ..Song::new("") });
        let found = if t <= 7 || t == 0x16 {
            m.unassigned.iter().copied().find(|&u| first(&m.songs[u]) <= 7 || first(&m.songs[u]) == 0x16)
        } else {
            m.unassigned.iter().copied().find(|&u| (8..0x10).contains(&first(&m.songs[u])))
        };
        if let Some(song) = found {
            m.log.push(format!(" * generalized replacement with {} song, with categories: {}", m.songs[song].name, categories_text(&m.songs[song].categories)));
            return Self::assign(m, slot, song, "APROX");
        }
        // Use a song twice.
        if let Some(s) = m.songs.iter().find(|s| intersects(&s.categories, &slot.categories)).cloned() {
            m.log.push(format!(" * double dipping with song {}, with categories: {}", s.name, categories_text(&s.categories)));
            m.log.push(format!("{:<40} {:>10} -> {}", slot.name, "COPY", s.name));
            m.songs.push(Song { replaces: slot.replaces, samples: None, ..s });
            return Ok(());
        }
        Err(BuildError::Data(format!("Not enough music to randomize this seed: nothing fits the [{}] slot ({:X})", slot.name, slot.replaces)))
    }

    /// Background and combat music share a buffer; keep their sum in it
    /// (`CheckBGMCombatMusicBudget`).
    fn check_music_budget(&mut self, m: &mut Music, random: &mut DotNetRandom) -> Result<(), BuildError> {
        let bgm_slots: Vec<Slot> = m.slots.iter().filter(|s| s.categories.contains(&0) || s.categories.contains(&2)).cloned().collect();
        let used_bgm: Vec<usize> = bgm_slots.iter().filter_map(|slot| m.songs.iter().position(|s| s.replaces == slot.replaces)).collect();
        let mut combat_first = random.next_below(2) == 1;
        let mut combat = m.songs.iter().position(|s| s.replaces == 0x1A && s.name != "mm-combat");
        if combat.is_none() {
            combat_first = true;
            combat = m.songs.iter().position(|s| s.replaces == 0x18 && s.name != "mm-fileselect");
        } else if m.songs.iter().any(|s| s.name.contains("songtest")) {
            combat_first = false;
        }
        if self.cosmetic("DisableCombatMusic").as_bool() == Some(true) {
            combat_first = false;
        }
        let combat = combat.ok_or_else(|| BuildError::Data("music: no combat music placed".into()))?;
        let combat_song = m.songs[combat].clone();
        let combat_size = self.sequence_size(&combat_song)?;
        m.log.push(format!(" SECOND PASS: Scanning for oversized BGM or combat cointoss: ({})", if combat_first { "COMBAT" } else { "BGM" }));
        if combat_first {
            if used_bgm.is_empty() {
                return Ok(());
            }
            m.bgm_budget = BUDGET - combat_size;
            m.log.push(format!(" new BGM budget: {:X}, from combat size: {:X}", m.bgm_budget, combat_size));
            for seq in used_bgm {
                let song = m.songs[seq].clone();
                if self.sequence_size(&song)? > m.bgm_budget {
                    m.log.push(format!("BGM sequence {} was too big to match your combat music, replacing ... ", song.name));
                    let slot = m.slots.iter().find(|s| s.replaces == song.replaces).cloned().ok_or_else(|| BuildError::Data("music: lost a slot".into()))?;
                    m.songs[seq].replaces = -1;
                    if !self.search_for_song(m, &slot, random)? {
                        return Err(BuildError::Data("Music budget: this seed can't find small enough music for a background slot. Try another!".into()));
                    }
                }
            }
        } else {
            let mut largest = 0;
            for &seq in &used_bgm {
                let song = m.songs[seq].clone();
                largest = largest.max(self.sequence_size(&song)?);
            }
            m.log.push(format!(" new Combat budget: {:X} from bgm size: {:X}", m.combat_budget, largest));
            m.combat_budget = BUDGET - largest;
            if combat_size > m.combat_budget {
                m.log.push(format!("Combat sequence {} was too big to match your BGM music, replacing ... ", combat_song.name));
                let slot = m.slots.iter().find(|s| s.name == "mm-combat").cloned().ok_or_else(|| BuildError::Data("music: lost the combat slot".into()))?;
                m.songs[combat].replaces = -1;
                if !self.search_for_song(m, &slot, random)? {
                    return Err(BuildError::Data("Music budget: this seed can't find small enough music for the combat slot. Try another!".into()));
                }
            }
        }
        Ok(())
    }

    /// Music plando: songs chosen for slots (`*MusicPlando.json` in the music
    /// folder; `PlandoUtils.GetRandomizedSongPlacements`).
    fn music_plando(&mut self, m: &mut Music, random: &mut DotNetRandom) -> Result<(), BuildError> {
        #[derive(serde::Deserialize, Default)]
        #[serde(rename_all = "PascalCase", default)]
        struct Combo {
            name: String,
            songs_list: Option<Vec<String>>,
            slots_list: Option<Vec<String>>,
            item_draw_count: Option<i64>,
        }
        let mut files: Vec<&MusicFile> = self.music.iter().filter(|f| f.name.to_ascii_lowercase().ends_with("musicplando.json")).collect();
        files.sort_by(|a, b| a.name.to_uppercase().cmp(&b.name.to_uppercase()));
        let mut placed: Vec<(usize, Slot)> = Vec::new();
        for f in files {
            let text = String::from_utf8_lossy(&f.data);
            let combos: Vec<Combo> = serde_json::from_str(text.trim_start_matches('\u{FEFF}')).map_err(|e| BuildError::Data(format!("Music plando: {} can't be read: {e}", f.name)))?;
            for c in combos {
                let (Some(songs), Some(slots)) = (c.songs_list, c.slots_list) else {
                    return Err(BuildError::Data(format!("Music plando: {} has a broken song or slot list for [{}]", f.name, c.name)));
                };
                let mut songs = shuffle(songs, random);
                let mut slots = shuffle(slots, random);
                songs.retain(|s| !placed.iter().any(|(i, _)| m.songs[*i].name == *s));
                for s in &songs {
                    if !m.songs.iter().any(|x| x.name == *s) {
                        return Err(BuildError::Data(format!("Music plando: no song called [{s}] (misspelled?)")));
                    }
                }
                slots.retain(|s| !placed.iter().any(|(_, p)| p.name == *s));
                for s in &slots {
                    if !m.slots.iter().any(|x| x.name == *s) {
                        return Err(BuildError::Data(format!("Music plando: no slot called [{s}] (misspelled?)")));
                    }
                }
                if songs.is_empty() || slots.is_empty() {
                    return Err(BuildError::Data(format!("Music plando: [{}] has nothing left to place", c.name)));
                }
                let draw = match c.item_draw_count {
                    Some(n) if n >= 0 && (n as usize) <= songs.len() => n as usize,
                    _ => songs.len(),
                };
                for i in 0..draw.min(slots.len()) {
                    let song = m.songs.iter().position(|x| x.name == songs[i]).unwrap();
                    let slot = m.slots.iter().find(|x| x.name == slots[i]).cloned().unwrap();
                    placed.push((song, slot));
                }
            }
        }
        for (song, slot) in placed {
            Self::assign(m, &slot, song, "PLANDO")?;
            m.slots.retain(|s| s.name != slot.name);
        }
        Ok(())
    }

    /// For music makers: a song with "songtest" in its name plays almost
    /// everywhere (`CheckSongTest`).
    fn check_song_test(&mut self, m: &mut Music) -> Result<(), BuildError> {
        let Some(test) = m.songs.iter().position(|s| s.name.contains("songtest")) else { return Ok(()) };
        let file_select = m.slots.iter().find(|s| s.name.contains("mm-fileselect")).cloned().ok_or_else(|| BuildError::Data("music: no file select slot".into()))?;
        Self::assign(m, &file_select, test, "SONGTEST")?;
        let song = m.songs[test].clone();
        let size = self.sequence_size(&song)?;
        let c = &song.categories;
        if c.contains(&5) || c.contains(&0x1A) {
            m.combat_budget = size;
            m.bgm_budget = BUDGET - size;
        } else if !(c.contains(&8) || c.contains(&9) || c.contains(&0x10) || c.contains(&0x16)) {
            m.bgm_budget = size;
            m.combat_budget = BUDGET - size;
        }
        Self::pointerize(m, 0x76, 0x18);
        Self::pointerize(m, 0x15, 0x18);
        let matching: Vec<i32> = m.slots.iter().filter(|s| intersects(&s.categories, c) && s.name != file_select.name).map(|s| s.replaces).collect();
        for slot in matching {
            Self::pointerize(m, slot, 0x18);
        }
        if let Some(p) = m.slots.iter().position(|s| s.name == file_select.name) {
            m.slots.remove(p);
        }
        if song.binaries.is_none() {
            return Ok(());
        }
        // Shops play vanilla songs that share the test song's bank, to check
        // its old instruments still work.
        let shared: Vec<usize> = (0..m.songs.len()).filter(|&i| i != test && m.songs[i].instrument == song.instrument && m.songs[i].binaries.is_none()).collect();
        let h = self.hash.clone().unwrap_or_default();
        let mut shared = shuffle(shared, &mut DotNetRandom::new(i32::from_le_bytes([h[0], h[1], h[2], h[3]]) ^ 0x5EED));
        for (scene, room, actor, music_at) in [(1334usize, 1335usize, 0x98usize, 0x7usize), (1158, 1159, 0x88, 0x7), (1188, 1189, 0x88, 0x7), (1502, 1503, 0xC4, 0x7)] {
            if !shared.is_empty() && !m.pointerized.is_empty() {
                let valid = shared.remove(0);
                let new_slot = m.pointerized.remove(0).previous_slot;
                m.songs[valid].replaces = new_slot;
                m.unassigned.retain(|&u| u != valid);
                m.log.push(format!(" -- ^ -- Instrument set number {:02X} also used by {}", m.songs[valid].instrument, m.songs[valid].name));
                self.rom.file(scene)?[music_at] = new_slot as u8;
            }
            let r = self.rom.file(room)?;
            r[actor] = 0xFF;
            r[actor + 1] = 0xFF;
        }
        Ok(())
    }

    /// `SequenceUtils.RebuildAudioSeq`.
    fn rebuild_audio_seq(&mut self, m: &mut Music) -> Result<(), BuildError> {
        let f = self.rom.index_of(SEQ_TABLE).unwrap();
        self.rom.open(f)?;
        let base = self.rom.files[f].addr;
        let table_at = |i: u32| (SEQ_TABLE + i * 16 - base) as usize;
        self.rom.open(AUDIOSEQ_FILE)?;
        let audioseq = self.rom.files[AUDIOSEQ_FILE].data.clone();
        let list = &mut m.songs;

        // The vanilla sequences: (address, data). Size 0 means the slot
        // points at another sequence (its "address" is that slot).
        let mut old: Vec<(u32, Option<Vec<u8>>)> = Vec::new();
        for i in 0..128u32 {
            let d = &self.rom.files[f].data;
            let addr = be32(d, table_at(i));
            let size = be32(d, table_at(i) + 4) as usize;
            let mut data = None;
            if size > 0 {
                data = Some(audioseq[addr as usize..addr as usize + size].to_vec());
            } else if let Some(j) = list.iter().position(|s| s.replaces == i as i32) {
                if addr > 0 && addr < 128 {
                    if list[j].replaces != 0x28 {
                        list[j].replaces = addr as i32;
                    } else {
                        data = old[0x18].1.clone();
                    }
                }
            }
            old.push((addr, data));
        }

        let mut new: Vec<(u32, Option<Vec<u8>>)> = Vec::new();
        let mut addr = 0u32;
        let mut audio: Vec<u8> = Vec::new();
        for i in 0..128usize {
            let old_size = old[i].1.as_ref().map_or(0, Vec::len);
            let mut entry_addr = if old_size == 0 { old[i].0 } else { addr };
            let pointer = m.pointerized.iter().find(|p| p.previous_slot == i as i32);
            let data = if let Some(p) = pointer {
                entry_addr = p.replaces as u32;
                None
            } else {
                match list.iter().position(|s| s.replaces == i as i32) {
                    Some(j) if list[j].mm_seq != -1 => old[list[j].mm_seq as usize].1.clone(),
                    Some(j) if list[j].binaries.as_ref().is_some_and(|b| !b.is_empty()) => Some(list[j].binaries.as_ref().unwrap()[0].data.to_vec()),
                    Some(j) => {
                        let mut d = match &list[j].file {
                            Some(d) => d.to_vec(),
                            None if list[j].name == "mmr_f_sot" => include_bytes!("../../../../data/rom/music/mmr_f_sot.bin").to_vec(),
                            None => return Err(BuildError::Data(format!("music not found: {}", list[j].name))),
                        };
                        // Sequences ripped from other games need MM's type byte.
                        if d.len() > 1 {
                            d[1] = 0x20;
                        }
                        Some(d)
                    }
                    None => old[i].1.clone(),
                }
            };
            let data = data.map(|mut d| {
                let padding = 0x10 - d.len() % 0x10;
                if padding != 0x10 {
                    d.resize(d.len() + padding, 0);
                }
                d
            });
            if let Some(d) = &data {
                audio.extend_from_slice(d);
                addr += d.len() as u32;
            }
            new.push((entry_addr, data));
        }

        let index = self.rom.append_file(audio, false);
        self.apply_mod("mods", "reloc_audio")?;
        // Point the sequence loader at the new file (`RelocateSeq`).
        let offset = (0xF8B0 + index * 0x10 + 8) & 0xFFFF;
        self.rom.write(0x00C2739C, &[0x3C, 0x08, 0x80, 0x0A, 0x8D, 0x05, (offset >> 8) as u8, offset as u8])?;
        let old_file = &mut self.rom.files[AUDIOSEQ_FILE];
        old_file.data = Vec::new();
        old_file.addr = old_file.end;
        old_file.cmp_addr = None;
        old_file.cmp_end = u32::MAX;

        let f = self.rom.index_of(SEQ_TABLE).unwrap();
        self.rom.open(f)?;
        for (i, (a, d)) in new.iter().enumerate() {
            let at = table_at(i as u32);
            let size = d.as_ref().map_or(0, Vec::len) as u32;
            self.rom.files[f].data[at..at + 4].copy_from_slice(&a.to_be_bytes());
            self.rom.files[f].data[at + 4..at + 8].copy_from_slice(&size.to_be_bytes());
        }

        // Each sequence's instrument set, form mask and name.
        let f = self.rom.index_of(INST_SET_MAP).unwrap();
        self.rom.open(f)?;
        let base = (INST_SET_MAP - self.rom.files[f].addr) as usize;
        for (i, (a, d)) in new.iter().enumerate() {
            let at = base + i * 2 + 2;
            let target = if d.as_ref().map_or(0, Vec::len) == 0 { *a as i32 } else { i as i32 };
            let song = list.iter().find(|s| s.replaces == target);
            if let Some(s) = song {
                self.rom.files[f].data[at] = s.instrument as u8;
            }
            if let Some(mask_file) = self.sequence_mask_file {
                let mut mask = song.and_then(|s| s.binaries.as_ref()?.first()?.form_mask.clone()).unwrap_or_else(|| vec![0xFF; 0x20]);
                mask.resize(SEQUENCE_DATA_SIZE, 0);
                self.rom.files[mask_file].data[i * SEQUENCE_DATA_SIZE..(i + 1) * SEQUENCE_DATA_SIZE].copy_from_slice(&mask);
            }
            if let Some(names_file) = self.sequence_names_file {
                let mut name = song.map(|s| track_name(&s.name)).unwrap_or_default();
                if name.chars().count() > SEQUENCE_NAME_MAX_SIZE - 1 {
                    name = name.chars().take(SEQUENCE_NAME_MAX_SIZE - 4).collect::<String>() + "...";
                }
                let mut bytes: Vec<u8> = name.chars().map(|c| if c.is_ascii() { c as u8 } else { b'?' }).collect();
                bytes.push(0);
                bytes.resize(SEQUENCE_NAME_MAX_SIZE, 0);
                self.rom.files[names_file].data[i * SEQUENCE_NAME_MAX_SIZE..(i + 1) * SEQUENCE_NAME_MAX_SIZE].copy_from_slice(&bytes);
            }
        }

        // For the spoiler log: what plays where.
        if self.cosmetic("Music").as_str() == Some("Random") {
            let slot_names: Vec<(i32, String)> = include_str!("../../../../data/rom/music/SEQS.txt")
                .split('\n')
                .map(|l| l.trim_end_matches('\r'))
                .collect::<Vec<_>>()
                .windows(4)
                .filter(|w| w[0].starts_with("mm-"))
                .filter_map(|w| music::hex(w[3]).ok().map(|s| (s, w[0].to_string())))
                .collect();
            let mut placements = Vec::new();
            for (slot, name) in slot_names {
                if let Some(s) = list.iter().find(|s| s.replaces == slot) {
                    if !placements.iter().any(|p: &SongPlacement| p.slot == name) {
                        placements.push(SongPlacement { slot: name, song: track_name(&s.name) });
                    }
                }
            }
            self.music_placements = placements;
        }
        Ok(())
    }

    /// Custom instrument samples go in one new file (`WriteNewSoundSamples`).
    fn write_new_sound_samples(&mut self, m: &Music) {
        let mut data: Vec<u8> = Vec::new();
        let mut written: Vec<(i64, u32)> = Vec::new();
        let mut patch = SamplePatch::default();
        for bank in &m.banks {
            let bank = bank.borrow();
            let mut samples = Vec::new();
            if let Some(list) = &bank.samples {
                for s in list.iter() {
                    let mut s = s.borrow_mut();
                    match written.iter().find(|(h, _)| *h == s.hash) {
                        Some(&(_, addr)) => s.addr = addr,
                        None => {
                            s.addr = data.len() as u32;
                            data.extend_from_slice(&s.data);
                            // (Padded by the remainder, as the old code did.)
                            let remainder = data.len() % 0x10;
                            if remainder > 0 {
                                data.resize(data.len() + remainder, 0);
                            }
                            written.push((s.hash, s.addr));
                        }
                    }
                    let addr = written.iter().find(|(h, _)| *h == s.hash).unwrap().1;
                    samples.push((s.marker, addr));
                }
            }
            patch.banks.push((bank.data.len(), samples));
        }
        patch.samples_file = self.rom.append_file(data, false);
        self.sample_patch = Some(patch);
    }

    /// `SequenceUtils.RebuildAudioBank`.
    fn rebuild_audio_bank(&mut self, m: &Music) -> Result<(), BuildError> {
        let fid = self.rom.index_of(NEW_INSTRUMENT_SET_ADDRESS).unwrap();
        self.rom.open(fid)?;
        let table = (NEW_INSTRUMENT_SET_ADDRESS - self.rom.files[fid].addr) as usize;
        let mut offset = 0u32;
        let mut data: Vec<u8> = Vec::new();
        for (index, bank) in m.banks.iter().enumerate().take(m.free_bank as usize + 1) {
            let bank = bank.borrow();
            data.extend_from_slice(&bank.data);
            let at = table + index * 16;
            let t = &mut self.rom.files[fid].data;
            t[at..at + 4].copy_from_slice(&offset.to_be_bytes());
            t[at + 4..at + 8].copy_from_slice(&(bank.data.len() as u32).to_be_bytes());
            for (b, v) in bank.meta.iter().take(8).enumerate() {
                t[at + 8 + b] = *v;
            }
            offset += bank.data.len() as u32;
            // As the old code: pad by the remainder plus a spare line.
            let remainder = bank.data.len() as u32 % 0x10;
            if remainder > 0 {
                data.resize(data.len() + remainder as usize + 0x10, 0);
                offset += remainder;
            }
        }
        let f = self.rom.index_of(AUDIOBANK).unwrap();
        self.rom.open(f)?;
        let file = &mut self.rom.files[f];
        file.end = file.addr + data.len() as u32;
        file.data = data;
        Ok(())
    }
}

/// The name shown in game for a song (without extension, comment or
/// "songforce" marker).
fn track_name(name: &str) -> String {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    let mut n = stem.to_string();
    if ext == ".zseq" {
        n = n.split('_').next().unwrap_or("").to_string();
    }
    if n.contains("songforce") {
        // `Regex.Replace(name, "(\W|^)songforce\W", "")`, then any left over.
        let chars: Vec<char> = n.chars().collect();
        let word = |c: char| c.is_alphanumeric() || c == '_';
        let mut out = String::new();
        let mut i = 0;
        while i < chars.len() {
            let rest: String = chars[i..].iter().collect();
            let lead = if i == 0 && rest.starts_with("songforce") { Some(0) } else if !word(chars[i]) && rest[chars[i].len_utf8()..].starts_with("songforce") { Some(1) } else { None };
            if let Some(l) = lead {
                let after = i + l + 9;
                if after < chars.len() && !word(chars[after]) {
                    i = after + 1;
                    continue;
                }
            }
            out.push(chars[i]);
            i += 1;
        }
        n = out.replace("songforce", "");
    }
    n
}

/// Point custom-sample banks at where their samples ended up, once the ROM
/// is laid out (`UpdateBankInstrumentPointers`).
pub(crate) fn update_bank_instrument_pointers(rom: &mut [u8], files: &[mmr_rom::fs::RomFile], patch: &SamplePatch) {
    let soundbank = files[5].cmp_addr.unwrap_or(0) as i64;
    let samples_at = files[patch.samples_file].cmp_addr.unwrap_or(0) as i64;
    let mut bank_at = files[3].cmp_addr.unwrap_or(0) as usize;
    for (len, samples) in &patch.banks {
        for &(marker, addr) in samples {
            let offset = (samples_at + addr as i64 - soundbank) as i32;
            let (m, o) = (marker.to_be_bytes(), offset.to_be_bytes());
            for i in 0..len.saturating_sub(4) {
                let at = bank_at + i;
                if rom.get(at..at + 4) == Some(&m[..]) {
                    rom[at..at + 4].copy_from_slice(&o);
                }
            }
        }
        bank_at += len;
    }
}
