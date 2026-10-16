//! Quick text, minigame speedups, enemies and shortened cutscenes
//! (`WriteQuickText`, `WriteSpeedUps`, `WriteEnemies`, `WriteCutscenes`).

use crate::enums;
use crate::message::{message, MessageEntry};
use crate::plan::has_flag;
use crate::{BuildError, Builder};
use serde_json::Value;

impl Builder<'_> {
    pub(crate) fn write_quick_text(&mut self) -> Result<(), BuildError> {
        if self.plan.flag("QuickTextEnabled") {
            self.apply_mod("mods", "quick_text")?;
        }
        Ok(())
    }

    pub(crate) fn write_speedups(&mut self) -> Result<(), BuildError> {
        let p = self.plan;
        if p.flag("SpeedupBeavers") {
            self.apply_mod("mods", "speedup_beavers")?;
            self.messages.update(message(0x10D6, None, |it| {
                it.play_sound_effect(0x291A).compile_time_wrap(|w| {
                    w.text("There's a total of ").red("25 rings");
                    w.text(". You must swim through them in the right order for it to count. Swim through the ring that's ");
                    w.red("flashing").text(".");
                });
                it.end_text_box();
                it.compile_time_wrap_text("My big brother will show you the way, so follow him and don't get separated!");
                it.end_final_text_box();
            }));
            self.messages.update(message(0x10FA, None, |it| {
                it.play_sound_effect(0x2919).text("This time, the limit is ").red("1:50").text(".").end_text_box();
                it.text("Don't fall behind!").end_final_text_box();
            }));
            self.messages.update(message(0x1107, None, |it| {
                it.play_sound_effect(0x2919).compile_time_wrap(|w| {
                    w.text("This time around, you have to beat ").red("1:40").text(".");
                });
                it.end_text_box().text("Don't fall behind!").end_final_text_box();
            }));
        }
        if p.flag("SpeedupDampe") {
            self.apply_mod("mods", "speedup_dampe")?;
        }
        if p.flag("SpeedupLabFish") {
            self.apply_mod("mods", "speedup_labfish")?;
        }
        if p.flag("SpeedupDogRace") {
            self.apply_mod("mods", "speedup_dograce")?;
        }
        if p.flag("SpeedupBank") {
            self.apply_mod("mods", "speedup_bank")?;
            self.messages.update(message(0x45C, None, |it| {
                it.quick_text(|it| {
                    it.text("What's this? You've already saved").new_line().text("up ").red("500 Rupees").text("!?!");
                });
                it.end_text_box();
                it.text("Well, little guy, here's your special").new_line();
                it.text("gift. Take it!").end_conversation().end_final_text_box();
            }));
            self.messages.update(message(0x45D, None, |it| {
                it.quick_text(|it| {
                    it.text("What's this? You've already saved").new_line().text("up ").red("1000 Rupees").text("?!");
                });
                it.end_text_box();
                it.text("Well, little guy, I can't take any").new_line();
                it.text("more deposits. Sorry, but this is").new_line();
                it.text("all I can give you.").end_conversation().end_final_text_box();
            }));
        }
        if p.flag("SpeedupBabyCuccos") {
            self.apply_mod("mods", "speedup_babycucco_minimap")?;
        }
        Ok(())
    }

    pub(crate) fn write_cutscenes(&mut self) -> Result<(), BuildError> {
        // Each chosen "shorten cutscene" option applies its patches, in the
        // order of the settings (General, then BossIntros) and enum values.
        for (key, table) in [("General", enums::shorten_cutscene_general()), ("BossIntros", enums::shorten_cutscene_boss_intro())] {
            let chosen = flags_value(table, self.plan.nested("ShortenCutsceneSettings", key));
            let mut values: Vec<_> = table.values.iter().filter(|v| v.value != 0).collect();
            values.sort_by_key(|v| v.value as u64);
            for v in values {
                if chosen & v.value == v.value {
                    for a in v.attributes.iter().filter(|a| a.kind == "HackContent") {
                        if let Some(name) = a.args.first().and_then(Value::as_str) {
                            self.apply_mod("mods", name)?;
                        }
                    }
                }
            }
        }
        if self.cutscene_general("FasterBankText") && self.plan.flag("UpdateNPCText") {
            self.apply_mod("mods", "faster_bank_text_restore_intro")?;
        }
        if self.cutscene_general("AutomaticCredits") {
            self.automatic_credits();
        }
        if !self.cutscene_general("ShortChestOpening") && self.plan.flag("UpdateChests") {
            self.apply_mod("mods", "update_chest_cutscene")?;
            // Replace the fairy revive cutscene with the large chest opening.
            self.rom.write_u16(0xB3C000 + 0x12B2B2, 0xFFF6)?;
            self.insert_large_chest_cutscene()?;
        }
        Ok(())
    }

    /// Make the credits text advance on its own.
    fn automatic_credits(&mut self) {
        const NEWLINE: u16 = 0x11;
        for id in 0x1F5Fu16..=0x1F75 {
            let Some(mut msg) = self.messages.get(id).cloned() else { continue };
            if msg.message.contains(&0x1C) {
                continue;
            }
            let auto_close = |text: &[u16]| -> Vec<u16> {
                let lines = text.iter().filter(|&&c| c == NEWLINE).count() + 1;
                let mut out: Vec<u16> = text.iter().copied().filter(|&c| c != 0xBF).collect();
                out.extend([0x1C, 0x00, (lines * 0x20) as u16, 0xBF]);
                out
            };
            if msg.message.contains(&0x13) {
                let parts = split_units(&msg.message, &[0x11, 0x13, 0x12]);
                let mut next_id: Option<u16> = None;
                for j in (0..parts.len()).rev() {
                    let mut text = auto_close(&parts[j]);
                    let new_id = self.extra_messages.iter().map(|m| m.id).max().unwrap_or(0x9006) + 1;
                    let mut header = msg.header.unwrap_or(crate::message::DEFAULT_HEADER);
                    if let Some(n) = next_id {
                        header[3..5].copy_from_slice(&n.to_be_bytes());
                    }
                    if j > 0 {
                        if msg.message.first() == Some(&0x05) {
                            text.insert(0, 0x05);
                        }
                        self.extra_messages.push(MessageEntry { header: Some(header), ..MessageEntry::new(new_id, text) });
                    } else {
                        msg.message = text;
                        msg.header = Some(header);
                    }
                    next_id = Some(new_id);
                }
            } else {
                let lines = msg.message.iter().filter(|&&c| c == NEWLINE).count() + 1;
                let mut out = Vec::new();
                for &c in &msg.message {
                    if c == 0xBF {
                        out.extend([0x1C, 0x00, (lines * 0x20) as u16, 0xBF]);
                    } else {
                        out.push(c);
                    }
                }
                msg.message = out;
            }
            msg.message.retain(|&c| c != 0x15);
            self.messages.update(msg);
        }
    }

    /// Repurpose the fairy revive actor cutscene for large chests
    /// (`SceneUtils.InsertLargeChestCutscene`).
    fn insert_large_chest_cutscene(&mut self) -> Result<(), BuildError> {
        for scene in &mut self.scenes {
            for setup in &mut scene.setups {
                let Some(list) = setup.actor_cutscenes.as_mut() else {
                    return Err(BuildError::Data(format!("scene {} has a setup without actor cutscenes", scene.number)));
                };
                if let Some(index) = list.iter().position(|c| c.camera_index == -9) {
                    let c = &mut list[index];
                    c.unknown00 = 0x384;
                    c.length = 0x87;
                    c.camera_index = -10;
                    c.cutscene_index = -1;
                    c.sound = 0;
                    c.unknown0b = 1;
                    c.hud_fade = 0;
                    c.return_camera_type = 0;
                    c.letterbox = 0x20;
                    let bytes = c.to_bytes();
                    let at = setup.actor_cutscene_list_address.unwrap() + index * 0x10;
                    self.rom.file(scene.file)?[at..at + 0x10].copy_from_slice(&bytes);
                }
            }
        }
        Ok(())
    }
}

/// The numeric value of a flags setting ("A, B, C").
pub(crate) fn flags_value(table: &enums::Enum, v: &Value) -> i64 {
    table.values.iter().filter(|e| has_flag(v, &e.name)).fold(0, |acc, e| acc | e.value)
}

/// `string.Split(separator)` on code units.
pub(crate) fn split_units(s: &[u16], sep: &[u16]) -> Vec<Vec<u16>> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i + sep.len() <= s.len() {
        if &s[i..i + sep.len()] == sep {
            out.push(s[start..i].to_vec());
            i += sep.len();
            start = i;
        } else {
            i += 1;
        }
    }
    out.push(s[start..].to_vec());
    out
}
