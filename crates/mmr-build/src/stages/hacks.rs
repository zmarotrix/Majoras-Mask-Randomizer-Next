//! The first group of changes in the old `MakeROM`: title screen, file
//! select, fixes that always apply, and some rewritten text.

use crate::color::Rgb;
use crate::message::{message, Header, TextBox};
use crate::{be32, BuildError, Builder};

impl Builder<'_> {
    pub(crate) fn write_hacks(&mut self) -> Result<(), BuildError> {
        if self.plan.choice("LogicMode") != "Vanilla" {
            self.apply_mod("mods", "title_screen")?;
            self.write_title_screen()?;
            self.apply_mod("mods", "misc_changes")?;
            self.write_misc_text();
            self.apply_mod("mods", "cm_cs")?;
            self.apply_mod("mods", "fix_song_of_healing")?;
            self.write_file_select()?;
        }
        for name in ["init_file", "fix_deku_drowning", "fix_collectable_flags", "fix_great_bay_clear_mikau", "fix_deku_playground_softlock"] {
            self.apply_mod("mods", name)?;
        }
        if self.plan.flag("EnablePictoboxSubject") {
            self.write_pictograph_prompt_text();
            // Don't update pictobox flags until after the prompt.
            self.write_code_u32(0x8011_27D0, 0)?;
        }
        if self.cutscene_general("FasterBankText") {
            self.write_bank_prompt_text();
        }
        self.extra_messages.push(message(0x9002, None, |it| {
            it.text("Return to spawn?").new_line().start_green().text(" ").new_line().two_choices().text("Yes").new_line().text("No").end_final_text_box();
        }));
        self.write_archery_double_reward_text();
        self.write_bank_post_reward_text();
        self.write_royal_wallet_text();
        self.write_moon_child_denial_text();
        Ok(())
    }

    /// A flag in the "shorten cutscenes" general settings.
    pub(crate) fn cutscene_general(&self, flag: &str) -> bool {
        crate::plan::has_flag(self.plan.nested("ShortenCutsceneSettings", "General"), flag)
    }

    /// Rotate the title logo's hue (`WriteTitleScreen`).
    fn write_title_screen(&mut self) -> Result<(), BuildError> {
        let mut title = crate::resources::mods("title_screen").unwrap().to_vec();
        let rot = self.plan.title_logo_color as f32;
        for i in 0..144 * 64 {
            let p = i * 4 + 8;
            let c = Rgb::new(title[p], title[p + 1], title[p + 2]).rotate_hue(rot);
            title[p] = c.r;
            title[p + 1] = c.g;
            title[p + 2] = c.b;
        }
        let c = Rgb::new(title[0x1FE72], title[0x1FE73], title[0x1FE76]).rotate_hue(rot);
        title[0x1FE72] = c.r;
        title[0x1FE73] = c.g;
        title[0x1FE76] = c.b;
        self.apply_hack(&title)
    }

    /// File select colours (`WriteFileSelect`).
    fn write_file_select(&mut self) -> Result<(), BuildError> {
        self.apply_mod("mods", "file_select")?;
        let mut skybox = [0x91u8, 0x78, 0x9B, 0x28, 0x00, 0x28];
        for i in 0..2 {
            let c = Rgb::new(skybox[i * 3], skybox[i * 3 + 1], skybox[i * 3 + 2]).rotate_hue(self.plan.file_select_skybox as f32);
            skybox[i * 3..i * 3 + 3].copy_from_slice(&[c.r, c.g, c.b]);
        }
        let addrs = addresses(crate::resources::addresses("skybox_init").unwrap());
        for i in 0..3 {
            self.write_rom_addr(&addrs[i], &mut [skybox[i * 2], skybox[i * 2 + 1]])?;
        }
        let mut fs = [0x64u8, 0x96, 0xFF, 0x96, 0xFF, 0xFF, 0x64, 0xFF, 0xFF];
        for i in 0..3 {
            let c = Rgb::new(fs[i * 3], fs[i * 3 + 1], fs[i * 3 + 2]).rotate_hue(self.plan.file_select_color as f32);
            fs[i * 3..i * 3 + 3].copy_from_slice(&[c.r, c.g, c.b]);
        }
        let addrs = addresses(crate::resources::addresses("fs_colour").unwrap());
        for i in 0..9 {
            if i < 6 {
                self.write_rom_addr(&addrs[i], &mut [0x00, fs[i]])?;
            } else {
                self.write_rom_addr(&addrs[i], &mut [fs[i]])?;
            }
        }
        Ok(())
    }

    /// Write the same bytes at several addresses (`WriteROMAddr`). Addresses
    /// tagged 1 in the top nibble adjust the data first (and, as in the old
    /// code, the adjustment carries over to the following addresses).
    pub(crate) fn write_rom_addr(&mut self, addrs: &[u32], data: &mut [u8]) -> Result<(), BuildError> {
        for &a in addrs {
            if a >> 28 == 1 {
                data[0] = data[0].wrapping_add(0xA);
                data[1] = data[1].wrapping_sub(0x70);
            }
            self.rom.write(a & 0x0FFF_FFFF, data)?;
        }
        Ok(())
    }

    /// Write a word into the `code` file by RAM address (`WriteCodeUInt32`).
    pub(crate) fn write_code_u32(&mut self, vram: u32, value: u32) -> Result<(), BuildError> {
        const CODE_RAM_START: u32 = 0x800A_5AC0;
        const CODE_FILE: usize = 31;
        let offset = (vram - CODE_RAM_START) as usize;
        self.rom.file(CODE_FILE)?[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    fn write_pictograph_prompt_text(&mut self) {
        self.messages.update(message(0xF8, None, |it| {
            it.text("Keep this ").start_red().picture_subject().start_white().text("?").new_line();
            it.start_green().text(" ").new_line();
            it.two_choices().text("Yes").new_line().text("No").end_final_text_box().start_white();
        }));
    }

    fn write_archery_double_reward_text(&mut self) {
        self.messages.update(message(0x23E, Some(Header::new(TextBox::Standard).y(0).icon(0xFE)), |it| {
            it.text("Y'played so well, y've'rned").new_line();
            it.text("yuhself the ").red("grand prize").text("!").disable_text_skip2().end_final_text_box();
        }));
    }

    fn write_bank_post_reward_text(&mut self) {
        // Copy of message 0x47A without EndFinalTextBox (0xBF).
        self.messages.update(message(0x1B67, Some(Header::new(TextBox::Standard2).y(0).icon(0xFE)), |it| {
            it.text("See! Doesn't it hold more than").new_line();
            it.text("your old one? Fill it up and bring").new_line();
            it.text("it all in to deposit!").disable_text_skip2();
        }));
        // Copy of message 0x47B without EndFinalTextBox (0xBF).
        self.messages.update(message(0x1B77, Some(Header::new(TextBox::Standard2).y(0).icon(0xFE)), |it| {
            it.text("That's what they call interest!").disable_text_skip2();
        }));
    }

    fn write_bank_prompt_text(&mut self) {
        for (id, first) in [(0x450u16, "How much? How much? "), (0x46E, "How much do you want?")] {
            self.messages.update(message(id, None, |it| {
                it.text(first).new_line();
                it.text("\u{CC}").new_line();
                it.text("Set the amount with \u{BB} \u{B4} \u{B5}").new_line();
                it.text("and press \u{B0} to decide.").end_final_text_box();
            }));
        }
    }

    fn write_royal_wallet_text(&mut self) {
        // Icon from the Giant Wallet message.
        self.messages.update(message(0xB, Some(Header::new(TextBox::FaintBlue).y(1).icon(0x9)), |it| {
            it.text("You got a ").red("Royal Wallet").text("!").new_line();
            it.text("It can hold up to ").red("999 Rupees").text(".").end_final_text_box();
        }));
    }

    fn write_moon_child_denial_text(&mut self) {
        self.messages.update(message(0x21FD, Some(Header::new(TextBox::FaintBlue).y(1)), |it| {
            it.text("Oh, it seems you're not worthy...").new_line();
            it.text("But I'll give you another chance...").end_text_box();
            it.text("Are you ready...to go back?").new_line();
            it.start_green().text(" ").new_line().two_choices().text("Yes").new_line().text("No").end_final_text_box();
        }));
    }

    fn write_misc_text(&mut self) {
        let m = &mut self.messages;
        m.update(message(3108, None, |it| {
            it.text("Say...Did you come to have some").new_line();
            it.text("items fashioned?").disable_text_skip2().end_final_text_box();
        }));
        m.update(message(3130, None, |it| {
            it.text("Gabora, fetch our customer some").new_line();
            it.text("coffee, quick-like.").end_text_box();
            it.text("Now then, let me take a look at").new_line();
            it.text("our offers.").end_text_box();
            it.text("Hmmm...").disable_text_skip2().end_final_text_box();
        }));
        m.update(message(3131, None, |it| {
            it.text("All right... For today's ").red("hot deal").text(",").new_line();
            it.text("it will cost you ").pink("100 Rupees").text(". It'll").new_line();
            it.text("be ready at ").red("sunrise").text(".").end_text_box();
            it.text("So how about it? Wanna grab a").new_line();
            it.text("hot item for ").pink("100 Rupees").text("?").new_line();
            it.start_green().two_choices().text("I'll do it").new_line().text("No thanks").end_final_text_box();
        }));
        m.update(message(3133, None, |it| {
            it.text("This is a secret, but I'll let you in").new_line();
            it.text("on it... The strongest sword out").new_line();
            it.text("there was forged using ").color(crate::message::RED, |it| {
                it.text("gold").new_line().text("dust");
            });
            it.text(".... I made it! Me!").end_conversation().end_final_text_box();
        }));
        m.update(message(3134, None, |it| {
            it.text("Wanna grab a deal? ").new_line();
            it.start_green().text(" ").new_line().two_choices().text("Yes").new_line().text("No thanks").end_final_text_box();
        }));
        m.update(message(3140, None, |it| {
            it.text("Hey! It's gonna be ready ").color(crate::message::RED, |it| {
                it.text("tomorrow").new_line().text("morning");
            });
            it.text(". We'll take good care of").new_line();
            it.text("it, so don't worry.").disable_text_skip2().end_final_text_box();
        }));
        m.update(message(3141, None, |it| {
            it.text("Hey! For today's special product").new_line();
            it.text("we'll need to get hold of some ").new_line();
            it.red("gold dust").text(".").disable_text_skip2().end_final_text_box();
        }));
        m.update(message(3142, None, |it| {
            it.text("Why, if it isn't ").red("gold dust").text("! And it's").new_line();
            it.text("even top quality!!!").end_text_box();
            it.text("Why, even if I use it to craft").new_line();
            it.text("a nifty item, there'll still be some").new_line();
            it.text("left...").end_text_box();
            it.text("All right! Just for you, I'll do this").new_line();
            it.text("for free. But don't tell anyone!").disable_text_skip2().end_final_text_box();
        }));
        m.update(message(3147, None, |it| {
            it.text("To make our item for you today,").new_line();
            it.text("I'll need ").red("gold dust").text(", which just so").new_line();
            it.text("happens to be first prize at the").new_line();
            it.text("Goron racetrack.").end_text_box();
            it.text("If I can just get some gold dust...").new_line();
            it.text("and this is just between us...I can").new_line();
            it.text("make you the ").color(crate::message::RED, |it| {
                it.text("hottest item").new_line().text("in the lands");
            });
            it.text("... Really!!").end_conversation().end_final_text_box();
        }));
        m.update(message(3150, None, |it| {
            it.text("Huh? ").pause_text(0x10).text("Look, I'm working on").new_line();
            it.text("making this item for you. I'm").new_line();
            it.text("busy, so don't bother me.").end_conversation().end_final_text_box();
        }));
        m.update(message(3153, None, |it| {
            it.text("Behold! My skills in craftsmanship").new_line();
            it.text("are truly unrivalled!").disable_text_skip2().end_final_text_box();
        }));
        m.update(message(3155, None, |it| {
            it.text("Ah! My finest work!").new_line();
            it.text("The look in your eye, I can").new_line();
            it.text("tell you really wanted this!!").disable_text_skip2().end_final_text_box();
        }));
    }
}

/// Lists of addresses from an `addresses` resource (`GetAddresses`).
pub(crate) fn addresses(data: &[u8]) -> Vec<Vec<u32>> {
    let mut out = Vec::new();
    let mut i = 0;
    while data[i] != 0xFF {
        let count = be32(data, i) as usize;
        i += 4;
        out.push((0..count).map(|k| be32(data, i + k * 4)).collect());
        i += count * 4;
    }
    out
}
