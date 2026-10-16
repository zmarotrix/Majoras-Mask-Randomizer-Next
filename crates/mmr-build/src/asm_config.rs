//! Configuration read by the in-game payload: filled in while building and
//! written with the asm patch. Port of the old `MMRConfig`.

#[derive(Debug, Clone, Default)]
pub struct MmrConfig {
    pub rupee_repeatable_locations: Vec<u16>,

    pub location_bottle_red_potion: u16,
    pub location_bottle_gold_dust: u16,
    pub location_bottle_milk: u16,
    pub location_bottle_chateau: u16,

    pub location_sword_kokiri: u16,
    pub location_sword_razor: u16,
    pub location_sword_gilded: u16,

    pub location_magic_small: u16,
    pub location_magic_large: u16,

    pub location_wallet_adult: u16,
    pub location_wallet_giant: u16,
    pub location_wallet_royal: u16,

    pub location_bomb_bag_small: u16,
    pub location_bomb_bag_big: u16,
    pub location_bomb_bag_biggest: u16,

    pub location_quiver_small: u16,
    pub location_quiver_large: u16,
    pub location_quiver_largest: u16,

    pub location_lullaby: u16,
    pub location_lullaby_intro: u16,

    /// `TingleMap` flags.
    pub extra_starting_maps: u8,
    pub extra_starting_swamp_skull_tokens: u8,
    pub extra_starting_ocean_skull_tokens: u8,
    pub extra_starting_item_ids: Vec<u8>,

    pub items_to_return_ids: Vec<u16>,
}

/// `TingleMap` flag value by name.
pub fn tingle_map(name: &str) -> u8 {
    match name {
        "Town" => 1,
        "Swamp" => 2,
        "Mountain" => 4,
        "Ranch" => 8,
        "Ocean" => 16,
        "Canyon" => 32,
        _ => 0,
    }
}

impl MmrConfig {
    /// The struct layout the payload reads (`MMRConfigStruct.ToBytes`).
    pub fn to_bytes(&self, version: u32) -> Result<Vec<u8>, String> {
        if self.extra_starting_item_ids.len() > 0x10 {
            return Err("too many extra starting item ids".into());
        }
        if self.rupee_repeatable_locations.len() > 0x80 {
            return Err("too many rupee repeatable locations".into());
        }
        let mut w = Vec::new();
        let u16_ = |w: &mut Vec<u8>, v: u16| w.extend_from_slice(&v.to_be_bytes());
        w.extend_from_slice(&version.to_be_bytes());
        let mut rupee = self.rupee_repeatable_locations.clone();
        rupee.resize(0x80, 0);
        for v in &rupee {
            u16_(&mut w, *v);
        }
        u16_(&mut w, rupee.len() as u16);
        for v in [
            self.location_bottle_red_potion,
            self.location_bottle_gold_dust,
            self.location_bottle_milk,
            self.location_bottle_chateau,
            self.location_sword_kokiri,
            self.location_sword_razor,
            self.location_sword_gilded,
            self.location_magic_small,
            self.location_magic_large,
            self.location_wallet_adult,
            self.location_wallet_giant,
            self.location_wallet_royal,
            self.location_bomb_bag_small,
            self.location_bomb_bag_big,
            self.location_bomb_bag_biggest,
            self.location_quiver_small,
            self.location_quiver_large,
            self.location_quiver_largest,
            self.location_lullaby,
            self.location_lullaby_intro,
        ] {
            u16_(&mut w, v);
        }
        let maps_and_tokens = (self.extra_starting_maps as u32) | ((self.extra_starting_swamp_skull_tokens as u32) << 6) | ((self.extra_starting_ocean_skull_tokens as u32) << 11);
        u16_(&mut w, maps_and_tokens as u16);
        let mut ids = self.extra_starting_item_ids.clone();
        ids.resize(0x10, 0);
        w.extend_from_slice(&ids);
        u16_(&mut w, self.extra_starting_item_ids.len() as u16);
        let mut ret = self.items_to_return_ids.clone();
        ret.resize(0x1F, 0);
        for v in &ret {
            u16_(&mut w, *v);
        }
        u16_(&mut w, self.items_to_return_ids.len() as u16);
        Ok(w)
    }
}

/// Draw settings for NPCs wearing masks, which depend on what's placed at
/// their mask's location (`MaskConfigUtils`).
#[derive(Debug, Clone)]
pub struct MaskConfig {
    pub npc_kafei_draw_mask: u8,
    pub don_gero_goron_draw_mask: bool,
    pub postman_draw_hat: bool,
    pub draw_mask_of_truth: bool,
    pub draw_garo_mask: bool,
    pub draw_pendant_of_memories: bool,
}

impl Default for MaskConfig {
    fn default() -> Self {
        MaskConfig {
            npc_kafei_draw_mask: 0x05,
            don_gero_goron_draw_mask: true,
            postman_draw_hat: true,
            draw_mask_of_truth: true,
            draw_garo_mask: true,
            draw_pendant_of_memories: true,
        }
    }
}

impl MaskConfig {
    pub fn update(&mut self, location: &str, display_item: &str) {
        match location {
            "MaskKeaton" => {
                let get = if crate::items::item_category(display_item) == Some("Masks") {
                    crate::items::starting_item_value(display_item).unwrap_or(0)
                } else {
                    0
                };
                self.npc_kafei_draw_mask = match get {
                    0x36..=0x49 => get - 0x35,
                    0x32 => 0x18,
                    0x33 => 0x16,
                    0x34 => 0x17,
                    g if g > 0x32 => 0x15,
                    _ => 0,
                };
            }
            "MaskDonGero" => self.don_gero_goron_draw_mask = display_item == "MaskDonGero",
            "MaskPostmanHat" => self.postman_draw_hat = display_item == "MaskPostmanHat",
            "MaskTruth" => self.draw_mask_of_truth = display_item == "MaskTruth",
            "MaskGaro" => self.draw_garo_mask = display_item == "MaskGaro",
            "TradeItemPendant" => self.draw_pendant_of_memories = display_item == "TradeItemPendant",
            _ => {}
        }
    }
}
