//! The Ikana well Gibdos' requests (`WriteGibdoRequirements`) and prices
//! (`WritePrices`).

use crate::enums;
use crate::items::number_to_words;
use crate::message::{message, MessageEntry};
use crate::{BuildError, Builder};

impl Builder<'_> {
    pub(crate) fn write_gibdo_requirements(&mut self) -> Result<(), BuildError> {
        let table = enums::gibdo_items();
        for (i, req) in self.plan.gibdo_requirements.iter().enumerate() {
            let gibdo = table.attr(&req.item_required, "ItemGibdo").ok_or_else(|| BuildError::Data(format!("unknown Gibdo request {}", req.item_required)))?;
            let num = |k: &str| gibdo.get(k).as_i64().unwrap_or(0);
            let mut message_id = num("MessageId") as u16;
            let default_amount = table.attr(&req.item_required, "ItemGibdoAmount").and_then(|a| a.get("DefaultAmount").as_i64()).unwrap_or(1);
            let custom = gibdo.get("CustomMessage").as_str();
            if let Some(custom) = custom.filter(|_| message_id == 0 || req.amount as i64 != default_amount) {
                let max = self.extra_messages.iter().map(|m| m.id).max().ok_or_else(|| BuildError::Data("extra messages unexpectedly empty".into()))?;
                message_id = max + 1;
                let amount = req.amount;
                let entry: MessageEntry = message(message_id, None, |it| {
                    it.compile_time_wrap(|w| {
                        if custom.contains("[AMOUNT]") {
                            w.text(&custom.replace("[AMOUNT]", &number_to_words(amount as i64)));
                        } else {
                            w.text(custom);
                            if amount > 1 {
                                w.text(&format!(" Preferably {} of them...", number_to_words(amount as i64)));
                            }
                        }
                    });
                    it.disable_text_skip2().end_final_text_box();
                });
                self.extra_messages.push(entry);
            }
            let request_type = match table.attr(&req.item_required, "ItemGibdoRequestType").and_then(|a| a.get("RequestType").as_str()) {
                Some("Bottle") => 1,
                Some("QuestTrade") => 2,
                Some("Photo") => 3,
                Some("Mask") => 4,
                _ => 0, // Ammo, or none
            };
            let data = num("Data") as u16;
            let bytes = [num("ItemAction") as u8, num("Item") as u8, req.amount, request_type as u8, (message_id >> 8) as u8, message_id as u8, (data >> 8) as u8, data as u8];
            self.rom.write(0xF666B8 + i as u32 * 8, &bytes)?;
            // Spawn parameters for the Gibdos in the well.
            self.rom.write_u16(0x029B0000 + 0xDA, 0x28C)?;
            self.rom.write_u16(0x029D2000 + 0xB6, 0x22A)?;
            self.rom.write_u16(0x029DC000 + 0xD6, 0x1FB)?;
        }
        Ok(())
    }

    /// Randomized prices: in messages, their headers and the code
    /// (`WritePrices`).
    pub(crate) fn write_prices(&mut self, new: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        let costs = self.plan.message_costs.clone().unwrap_or_default();
        for (i, mc) in crate::prices::message_costs().iter().enumerate() {
            let Some(Some(cost)) = costs.get(i).copied() else { continue };
            for &(message_id, index) in &mc.message_ids {
                let old = self.messages.get(message_id).cloned().ok_or_else(|| BuildError::Data(format!("missing message {message_id}")))?;
                let pos = match new.iter().position(|m| m.id == message_id) {
                    Some(p) => p,
                    None => {
                        new.push(MessageEntry { header: old.header, ..MessageEntry::new(message_id, old.message.clone()) });
                        new.len() - 1
                    }
                };
                let m = &mut new[pos];
                let mut header = m.header.or(old.header).unwrap_or(crate::message::DEFAULT_HEADER);
                let at = 5 + index as usize * 2;
                let old_cost = i16::from_be_bytes([header[at], header[at + 1]]);
                if old_cost >= 0 {
                    header[at..at + 2].copy_from_slice(&cost.to_be_bytes());
                }
                m.header = Some(header);
                m.message = crate::prices::replace_price(&m.message, index as usize, cost, cost != 1 && message_id != 1143);
                if message_id == 1143 {
                    self.bank_withdraw_fee = cost;
                }
            }
            for &address in &mc.price_addresses {
                self.rom.write_u16(address, cost)?;
            }
            for &address in &mc.subtract_price_addresses {
                self.rom.write_u16(address, 0u16.wrapping_sub(cost))?;
            }
        }
        Ok(())
    }

    /// Item-related text (shops, world models, hints...). Written to the
    /// message table at the messages stage.
    pub(crate) fn write_item_texts(&mut self, new_messages: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        if self.plan.flag("UpdateShopAppearance") {
            self.write_shop_text(new_messages)?;
        }
        if self.plan.flag("UpdateWorldModels") {
            self.write_world_model_text(new_messages)?;
        }
        self.write_misc_item_text(new_messages)?;
        self.write_victory_condition_text(new_messages)?;
        self.write_custom_item_messages();
        if self.plan.flag("OathHint") {
            self.write_oath_hint_text(new_messages)?;
        }
        if self.plan.flag("RemainsHint") {
            self.write_remains_hint_text(new_messages)?;
        }
        if self.plan.flag("FairyAndSkullHint") {
            self.write_fairy_and_skull_hint_text(new_messages)?;
        }
        if self.plan.flag("UpdateNPCText") {
            self.write_npc_hint_text(new_messages)?;
        }
        self.write_dungeon_item_text(new_messages)
    }
}
