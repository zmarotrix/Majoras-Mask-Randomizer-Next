//! Gossip stone text, then writing the message table (`WriteGossipQuotes`
//! and `MessageTable.WriteDefault`).

use crate::message::{message, MessageEntry};
use crate::{BuildError, Builder};

impl Builder<'_> {
    pub(crate) fn write_messages(&mut self) -> Result<(), BuildError> {
        self.write_gossip_quotes()?;
        let transfers = self.messages.remove_transfers();
        self.extra_messages.extend(transfers);
        let quick_text = self.plan.flag("QuickTextEnabled");
        self.messages.write_default(&mut self.rom, quick_text)
    }

    fn write_gossip_quotes(&mut self) -> Result<(), BuildError> {
        let p = self.plan;
        if p.choice("LogicMode") == "Vanilla" {
            return Ok(());
        }
        if p.flag("FreeHints") {
            self.apply_mod("mods", "gossip_hints_free")?;
        }
        if p.flag("TolerantGossipStones") {
            self.apply_mod("mods", "gossip_hints_tolerant")?;
        }
        if p.flag("FreeGaroHints") {
            self.apply_mod("mods", "free_garo_hints")?;
            self.messages.update(message(0x24E, None, |it| {
                it.light_blue_with(|it| {
                    it.text("I can't see it, but I sense there's").new_line();
                    it.text("a thirst for blood looming all").new_line();
                    it.text("around us...");
                });
                it.end_final_text_box();
            }));
        }
        if p.choice("GossipHintStyle") != "Default" {
            for q in p.gossip_quotes.iter().flatten() {
                let header = q.header.as_ref().map(|h| <[u8; 11]>::try_from(h.as_slice()).map_err(|_| BuildError::Data("bad gossip header".into()))).transpose()?;
                self.messages.update(MessageEntry { header, ..MessageEntry::new(q.id, q.message.clone()) });
            }
        }
        Ok(())
    }
}
