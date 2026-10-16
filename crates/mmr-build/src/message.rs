//! In-game text: the message table and a builder for new messages. Port of
//! the old `MessageTable`, `MessageEntry` and `MessageEntryBuilder`.
//!
//! Message text is kept as UTF-16 code units, like the old code's strings;
//! each unit becomes one byte when written (control codes and characters
//! share the same 0-255 range).

use crate::BuildError;
use mmr_rom::Rom;

pub const MESSAGE_TABLE: (u32, u32) = (0xC5_D0D8, 0xC6_6040);
pub const MESSAGE_DATA_FILE: usize = 29;
pub const DEFAULT_HEADER: [u8; 11] = [0x00, 0x00, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];

/// Text as UTF-16 code units (`"\u{CC}"` is the byte 0xCC).
pub fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageEntry {
    pub id: u16,
    /// `None` keeps the existing message's header when updating.
    pub header: Option<[u8; 11]>,
    pub message: Vec<u16>,
    pub exclude_from_quick_text: bool,
    pub transfer_to_extended_table: bool,
}

impl MessageEntry {
    /// Leave this message's text speed alone.
    pub fn exclude_quick_text(mut self) -> Self {
        self.exclude_from_quick_text = true;
        self
    }
    /// Move this message to the extended table.
    pub fn transfer(mut self) -> Self {
        self.transfer_to_extended_table = true;
        self
    }

    pub fn new(id: u16, message: Vec<u16>) -> Self {
        MessageEntry { id, header: Some(DEFAULT_HEADER), message, exclude_from_quick_text: false, transfer_to_extended_table: false }
    }

    /// Size in bytes, padded to 4.
    pub fn size(&self) -> usize {
        (11 + self.message.len() + 3) & !3
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, BuildError> {
        if self.size() > 0x500 {
            return Err(BuildError::Data(format!("message {} is too large", self.id)));
        }
        let mut out = vec![0u8; self.size()];
        out[..11].copy_from_slice(&self.header.unwrap_or(DEFAULT_HEADER));
        for (i, c) in self.message.iter().enumerate() {
            out[11 + i] = *c as u8;
        }
        Ok(out)
    }

    fn from_bytes(id: u16, buf: &[u8], offset: usize, size: usize) -> Self {
        let header: [u8; 11] = buf[offset..offset + 11].try_into().unwrap();
        let body = &buf[offset + 11..offset + 11 + size];
        // Drop end padding.
        let len = body.iter().rposition(|&b| b != 0).map_or(size, |i| i + 1);
        MessageEntry { header: Some(header), ..MessageEntry::new(id, body[..len].iter().map(|&b| b as u16).collect()) }
    }
}

/// Messages by id, iterated in the same order as the old code's .NET
/// `Dictionary` (insertion order; a removed entry's slot is reused by the
/// next insert).
#[derive(Debug, Clone, Default)]
pub struct MessageTable {
    slots: Vec<Option<MessageEntry>>,
    free: Vec<usize>,
    max_entries: usize,
    max_data_length: Option<usize>,
}

impl MessageTable {
    pub fn new(max_entries: usize) -> Self {
        MessageTable { max_entries, ..Default::default() }
    }

    fn position(&self, id: u16) -> Option<usize> {
        self.slots.iter().position(|s| s.as_ref().is_some_and(|m| m.id == id))
    }

    pub fn len(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &MessageEntry> {
        self.slots.iter().flatten()
    }

    pub fn get(&self, id: u16) -> Option<&MessageEntry> {
        self.position(id).and_then(|i| self.slots[i].as_ref())
    }

    /// Add a message (`AddMessage`); its id must be new.
    pub fn add(&mut self, entry: MessageEntry) -> Result<(), BuildError> {
        if self.position(entry.id).is_some() {
            return Err(BuildError::Data(format!("message {:#X} added twice", entry.id)));
        }
        // .NET reuses the most recently freed slot first.
        match self.free.pop() {
            Some(i) => self.slots[i] = Some(entry),
            None => self.slots.push(Some(entry)),
        }
        Ok(())
    }

    /// Replace an existing message, keeping its header if the new one has
    /// none (`UpdateMessages`). Unknown ids are ignored, like the old code.
    pub fn update(&mut self, mut entry: MessageEntry) {
        if let Some(i) = self.position(entry.id) {
            if entry.header.is_none() {
                entry.header = self.slots[i].as_ref().unwrap().header;
            }
            self.slots[i] = Some(entry);
        }
    }

    pub fn update_all(&mut self, entries: impl IntoIterator<Item = MessageEntry>) {
        for e in entries {
            self.update(e);
        }
    }

    /// Take out messages marked for the extended table.
    pub fn remove_transfers(&mut self) -> Vec<MessageEntry> {
        let mut out = Vec::new();
        for i in 0..self.slots.len() {
            if self.slots[i].as_ref().is_some_and(|m| m.transfer_to_extended_table) {
                out.push(self.slots[i].take().unwrap());
                self.free.push(i);
            }
        }
        out
    }

    /// Strip text-speed commands so text appears instantly.
    pub fn apply_quick_text(&mut self) {
        for m in self.slots.iter_mut().flatten() {
            if !m.exclude_from_quick_text {
                m.message = strip_quick_text(&m.message);
            }
        }
    }

    /// Message table and data bytes (`BuildData`).
    pub fn build_data(&self) -> Result<(Vec<u8>, Vec<u8>), BuildError> {
        let count = self.len();
        if count > self.max_entries {
            return Err(BuildError::Data(format!("message table has {count} entries, the most is {}", self.max_entries)));
        }
        let total: usize = self.iter().map(MessageEntry::size).sum();
        let total = (total + 0xF) & !0xF;
        if self.max_data_length.is_some_and(|max| total > max) {
            return Err(BuildError::Data(format!("message data is bigger than {:#X} bytes", self.max_data_length.unwrap())));
        }
        let mut data = vec![0u8; total];
        let mut table = vec![0u8; (count + 1) * 8];
        let mut offset = 0usize;
        for (index, m) in self.iter().enumerate() {
            table[index * 8..index * 8 + 2].copy_from_slice(&m.id.to_be_bytes());
            table[index * 8 + 4..index * 8 + 8].copy_from_slice(&(offset as u32 | 0x0800_0000).to_be_bytes());
            let bytes = m.to_bytes()?;
            data[offset..offset + bytes.len()].copy_from_slice(&bytes);
            offset += bytes.len();
        }
        table[count * 8..count * 8 + 2].copy_from_slice(&0xFFFFu16.to_be_bytes());
        Ok((table, data))
    }

    /// Read the game's message table (`ReadDefault`).
    pub fn read_default(rom: &mut Rom) -> Result<Self, BuildError> {
        let (start, end) = MESSAGE_TABLE;
        let mut table = MessageTable::default();
        for i in 0..(end - start) / 8 {
            let addr = start + i * 8;
            let id = rom.read_u16(addr)?;
            let offset = (rom.read_u32(addr + 4)? & 0xFF_FFFF) as usize;
            let next = (rom.read_u32(addr + 0xC)? & 0xFF_FFFF) as usize;
            let size = if next > 0 { next.wrapping_sub(offset).wrapping_sub(11) } else { 5 };
            if id == 0xFFFF {
                break;
            }
            let data = &rom.files[MESSAGE_DATA_FILE].data;
            table.slots.push(Some(MessageEntry::from_bytes(id, data, offset, size)));
        }
        table.max_entries = table.slots.len();
        table.max_data_length = Some((rom.files[MESSAGE_DATA_FILE + 1].addr - rom.files[MESSAGE_DATA_FILE].addr) as usize);
        Ok(table)
    }

    /// Write the table back in place and replace the data file (`WriteDefault`).
    pub fn write_default(&mut self, rom: &mut Rom, quick_text: bool) -> Result<(), BuildError> {
        if quick_text {
            self.apply_quick_text();
        }
        let (table, data) = self.build_data()?;
        rom.write(MESSAGE_TABLE.0, &table)?;
        let file = &mut rom.files[MESSAGE_DATA_FILE];
        file.end = file.addr + data.len() as u32;
        file.data = data;
        Ok(())
    }
}

/// Remove pause (0x1F xx xx) and text-speed (0x17, 0x18) commands, except
/// where they're arguments of an earlier command (the old regex
/// `(?<!(?:\x1B|\x1C|\x1D|\x1E).?)(?:\x1F..|\x17|\x18)`).
pub fn strip_quick_text(s: &[u16]) -> Vec<u16> {
    let is_arg_cmd = |c: u16| (0x1B..=0x1E).contains(&c);
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let protected = (i >= 1 && is_arg_cmd(s[i - 1])) || (i >= 2 && is_arg_cmd(s[i - 2]));
        if !protected {
            if s[i] == 0x1F && i + 2 < s.len() {
                i += 3;
                continue;
            }
            if s[i] == 0x17 || s[i] == 0x18 {
                i += 1;
                continue;
            }
        }
        out.push(s[i]);
        i += 1;
    }
    out
}

/// .NET `char.IsWhiteSpace`.
fn is_white_space(c: u16) -> bool {
    matches!(c, 0x09..=0x0D | 0x20 | 0x85 | 0xA0 | 0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000)
}

fn has_width(c: u16) -> bool {
    const SPECIAL: [u16; 17] = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x09, 0x11, 0x12, 0x13, 0x19, 0x1E, 0xBF, 0xC2, 0xC3, 0xE0];
    !is_white_space(c) && !SPECIAL.contains(&c)
}

fn wrap_line(line: &[u16], width: usize) -> Vec<Vec<u16>> {
    let mut lines = Vec::new();
    let mut current: Vec<&[u16]> = Vec::new();
    let mut length = 0usize;
    for word in line.split(|&c| c == b' ' as u16) {
        let w = word.iter().filter(|&&c| has_width(c)).count();
        if length + w > width {
            lines.push(current.join(&(b' ' as u16)));
            length = 0;
            current.clear();
        }
        current.push(word);
        length += w + 1;
    }
    lines.push(current.join(&(b' ' as u16)));
    lines
}

/// Word-wrap to `width` columns, 4 lines per text box (`string.Wrap`).
pub fn wrap(text: &[u16], width: usize) -> Vec<u16> {
    const NEWLINE: u16 = 0x11;
    const END_TEXT_BOX: u16 = 0x10;
    let mut lines = Vec::new();
    for line in text.split(|&c| c == NEWLINE).filter(|l| !l.is_empty()) {
        lines.extend(wrap_line(line, width));
    }
    let boxes: Vec<Vec<u16>> = lines.chunks(4).map(|chunk| chunk.join(&NEWLINE)).collect();
    boxes.join(&END_TEXT_BOX)
}

/// Text box styles for [`Header`].
#[derive(Debug, Clone, Copy)]
pub enum TextBox {
    Standard = 0x00,
    SignPost = 0x01,
    FaintBlue = 0x02,
    OcarinaStaff = 0x03,
    Standard2 = 0x06,
}

/// Message header builder (`HeaderBuilder`).
#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub text_box: u8,
    pub y: u8,
    pub icon: u8,
    pub next_message: u16,
    pub rupee_cost: u16,
}

impl Default for Header {
    fn default() -> Self {
        Header { text_box: 0, y: 0, icon: 0xFE, next_message: 0xFFFF, rupee_cost: 0xFFFF }
    }
}

impl Header {
    pub fn new(text_box: TextBox) -> Self {
        Header { text_box: text_box as u8, ..Default::default() }
    }
    pub fn y(mut self, y: u8) -> Self {
        self.y = y;
        self
    }
    pub fn icon(mut self, icon: u8) -> Self {
        self.icon = icon;
        self
    }
    pub fn next_message(mut self, id: u16) -> Self {
        self.next_message = id;
        self
    }
    pub fn rupee_cost(mut self, cost: u16) -> Self {
        self.rupee_cost = cost;
        self
    }
    pub fn build(self) -> [u8; 11] {
        let [n1, n2] = self.next_message.to_be_bytes();
        let [c1, c2] = self.rupee_cost.to_be_bytes();
        [self.text_box, self.y, self.icon, n1, n2, c1, c2, 0xFF, 0xFF, 0xFF, 0xFF]
    }
}

pub const WHITE: u16 = 0x00;
pub const RED: u16 = 0x01;
pub const GREEN: u16 = 0x02;
pub const DARK_BLUE: u16 = 0x03;
pub const YELLOW: u16 = 0x04;
pub const LIGHT_BLUE: u16 = 0x05;
pub const PINK: u16 = 0x06;
pub const SILVER: u16 = 0x07;
pub const ORANGE: u16 = 0x08;

/// Message text builder (`MessageBuilder`). Methods chain:
/// `Msg::new().text("Hi").new_line().red("there").end_final_text_box()`.
#[derive(Debug, Clone, Default)]
pub struct Msg {
    pub text: Vec<u16>,
    colors: Vec<u16>,
}

impl Msg {
    pub fn new() -> Self {
        Msg::default()
    }

    pub fn push(&mut self, c: u16) -> &mut Self {
        self.text.push(c);
        self
    }
    /// Append a 16-bit value as two units (high byte first).
    pub fn push16(&mut self, v: u16) -> &mut Self {
        self.push(v >> 8).push(v & 0xFF)
    }
    pub fn text(&mut self, text: &str) -> &mut Self {
        self.text.extend(text.encode_utf16());
        self
    }
    pub fn units(&mut self, units: &[u16]) -> &mut Self {
        self.text.extend_from_slice(units);
        self
    }

    pub fn quick_text_start(&mut self) -> &mut Self {
        self.push(0x17)
    }
    pub fn quick_text_stop(&mut self) -> &mut Self {
        self.push(0x18)
    }
    pub fn quick_text(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        self.quick_text_start();
        f(self);
        self.quick_text_stop()
    }
    pub fn auto_wrap_start(&mut self) -> &mut Self {
        self.push(0x09).push(0x11)
    }
    pub fn auto_wrap_stop(&mut self) -> &mut Self {
        self.push(0x09).push(0x12)
    }
    pub fn runtime_wrap(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        self.auto_wrap_start();
        f(self);
        self.auto_wrap_stop()
    }
    pub fn runtime_wrap_text(&mut self, text: &str) -> &mut Self {
        self.runtime_wrap(|m| {
            m.text(text);
        })
    }
    /// Word-wrap at build time (`CompileTimeWrap`). The wrapped part shares
    /// this builder's colour stack.
    pub fn compile_time_wrap(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        let mut inner = Msg { text: Vec::new(), colors: std::mem::take(&mut self.colors) };
        f(&mut inner);
        self.colors = inner.colors;
        let wrapped = wrap(&inner.text, 35);
        self.units(&wrapped)
    }
    pub fn compile_time_wrap_text(&mut self, text: &str) -> &mut Self {
        let wrapped = wrap(&units(text), 35);
        self.units(&wrapped)
    }
    pub fn picture_subject(&mut self) -> &mut Self {
        self.push(0x09).push(0x13)
    }
    /// Start of a runtime-filled field for a get-item index: 3 name,
    /// 4 description, 5 article, 6 pronoun, 7 amount, 8 verb.
    pub fn runtime_start(&mut self, kind: u16, get_item_index: u16) -> &mut Self {
        self.push(0x09).push(kind).push16(get_item_index)
    }
    pub fn runtime_generic_stop(&mut self) -> &mut Self {
        self.push(0x09).push(0x02)
    }
    pub fn runtime_stray_fairy_locations_start(&mut self, get_item_indices: &[u16]) -> &mut Self {
        self.push(0x09).push(0x09);
        for &i in get_item_indices {
            self.push16(i);
        }
        self.push16(0xFFFF)
    }
    pub fn play_sound_effect(&mut self, id: u16) -> &mut Self {
        self.push(0x1E).push16(id)
    }
    pub fn pause_text(&mut self, frames: u16) -> &mut Self {
        self.push(0x1F).push16(frames)
    }
    pub fn end_text_box(&mut self) -> &mut Self {
        self.push(0x10)
    }
    pub fn reset_cursor(&mut self) -> &mut Self {
        self.push(0x13)
    }
    pub fn stray_fairy_count(&mut self) -> &mut Self {
        self.push(0x0C)
    }
    pub fn skulltula_count(&mut self) -> &mut Self {
        self.push(0x0D)
    }
    pub fn end_final_text_box(&mut self) -> &mut Self {
        self.push(0xBF)
    }
    pub fn two_choices(&mut self) -> &mut Self {
        self.push(0xC2)
    }
    pub fn three_choices(&mut self) -> &mut Self {
        self.push(0xC3)
    }
    pub fn end_conversation(&mut self) -> &mut Self {
        self.push(0xE0)
    }
    pub fn disable_text_skip(&mut self) -> &mut Self {
        self.push(0x15)
    }
    pub fn disable_text_skip2(&mut self) -> &mut Self {
        self.push(0x19)
    }
    pub fn disable_text_box_close(&mut self) -> &mut Self {
        self.push(0x1A)
    }
    pub fn new_line(&mut self) -> &mut Self {
        self.push(0x11)
    }

    pub fn push_color(&mut self, color: u16) -> &mut Self {
        self.colors.push(color);
        self.push(color)
    }
    pub fn pop_color(&mut self) -> &mut Self {
        self.colors.pop();
        let c = self.colors.last().copied().unwrap_or(WHITE);
        self.push(c)
    }
    pub fn start_white(&mut self) -> &mut Self {
        self.push_color(WHITE)
    }
    pub fn start_red(&mut self) -> &mut Self {
        self.push_color(RED)
    }
    pub fn start_green(&mut self) -> &mut Self {
        self.push_color(GREEN)
    }
    pub fn start_dark_blue(&mut self) -> &mut Self {
        self.push_color(DARK_BLUE)
    }
    pub fn start_yellow(&mut self) -> &mut Self {
        self.push_color(YELLOW)
    }
    pub fn start_light_blue(&mut self) -> &mut Self {
        self.push_color(LIGHT_BLUE)
    }
    pub fn start_pink(&mut self) -> &mut Self {
        self.push_color(PINK)
    }
    /// Text in a colour, then back to the previous colour.
    pub fn color(&mut self, color: u16, f: impl FnOnce(&mut Self)) -> &mut Self {
        self.push_color(color);
        f(self);
        self.pop_color()
    }
    pub fn color_text(&mut self, color: u16, text: &str) -> &mut Self {
        self.color(color, |m| {
            m.text(text);
        })
    }
    pub fn white(&mut self, text: &str) -> &mut Self {
        self.color_text(WHITE, text)
    }
    pub fn red(&mut self, text: &str) -> &mut Self {
        self.color_text(RED, text)
    }
    pub fn green(&mut self, text: &str) -> &mut Self {
        self.color_text(GREEN, text)
    }
    pub fn dark_blue(&mut self, text: &str) -> &mut Self {
        self.color_text(DARK_BLUE, text)
    }
    pub fn yellow(&mut self, text: &str) -> &mut Self {
        self.color_text(YELLOW, text)
    }
    pub fn light_blue(&mut self, text: &str) -> &mut Self {
        self.color_text(LIGHT_BLUE, text)
    }
    pub fn pink(&mut self, text: &str) -> &mut Self {
        self.color_text(PINK, text)
    }

    pub fn build(&self) -> Vec<u16> {
        self.text.clone()
    }

    /// Item name filled in at runtime for a location (`RuntimeItemName`).
    pub fn runtime_item_name(&mut self, name: &str, location: &str) -> &mut Self {
        self.runtime_start(3, gi(location)).text(name).runtime_generic_stop()
    }
    /// `RuntimeArticle`.
    pub fn runtime_article(&mut self, item: &str, location: &str, indefinite: Option<&str>) -> &mut Self {
        let a = crate::stages::article(item, indefinite);
        self.runtime_start(5, gi(location)).text(&a).runtime_generic_stop()
    }
    /// `RuntimePronoun`.
    pub fn runtime_pronoun(&mut self, item: &str, location: &str) -> &mut Self {
        self.runtime_start(6, gi(location)).text(crate::stages::pronoun(item)).runtime_generic_stop()
    }
    /// `RuntimePronounOrAmount`.
    pub fn runtime_pronoun_or_amount(&mut self, item: &str, location: &str) -> &mut Self {
        let t = crate::stages::pronoun_or_amount(item);
        self.runtime_start(7, gi(location)).text(&t).runtime_generic_stop()
    }
    /// `RuntimeVerb`.
    pub fn runtime_verb(&mut self, item: &str, location: &str) -> &mut Self {
        self.runtime_start(8, gi(location)).text(crate::stages::verb(item)).runtime_generic_stop()
    }
    /// `RuntimeItemDescription`.
    pub fn runtime_item_description(&mut self, item: &str, keeper: &str, location: &str) -> &mut Self {
        crate::stages::runtime_item_description(self, item, keeper, gi(location));
        self
    }
    pub fn start_white_text(&mut self) -> &mut Self {
        self.start_white()
    }
    pub fn start_red_text(&mut self) -> &mut Self {
        self.start_red()
    }
    pub fn start_green_text(&mut self) -> &mut Self {
        self.start_green()
    }
    pub fn start_dark_blue_text(&mut self) -> &mut Self {
        self.start_dark_blue()
    }
    pub fn start_yellow_text(&mut self) -> &mut Self {
        self.start_yellow()
    }
    pub fn start_light_blue_text(&mut self) -> &mut Self {
        self.start_light_blue()
    }
    pub fn start_pink_text(&mut self) -> &mut Self {
        self.start_pink()
    }
    /// Red text, or just the colour codes when there's no text (the old
    /// code's `Red(null)`).
    pub fn red_opt(&mut self, text: Option<&str>) -> &mut Self {
        self.color_text(RED, text.unwrap_or(""))
    }
    pub fn red_with(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        self.color(RED, f)
    }
    pub fn pink_with(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        self.color(PINK, f)
    }
    pub fn green_with(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        self.color(GREEN, f)
    }
    pub fn yellow_with(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        self.color(YELLOW, f)
    }
    pub fn light_blue_with(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        self.color(LIGHT_BLUE, f)
    }
    pub fn dark_blue_with(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        self.color(DARK_BLUE, f)
    }
    pub fn white_with(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        self.color(WHITE, f)
    }
}

fn gi(location: &str) -> u16 {
    crate::items::get_item_index(location).unwrap_or_else(|| panic!("{location} has no get-item index"))
}

/// A new message (`MessageEntryBuilder`): id, optional header, text.
pub fn message(id: u16, header: Option<Header>, f: impl FnOnce(&mut Msg)) -> MessageEntry {
    let mut m = Msg::new();
    f(&mut m);
    MessageEntry {
        id,
        header: header.map(Header::build),
        message: m.text,
        exclude_from_quick_text: false,
        transfer_to_extended_table: false,
    }
}
