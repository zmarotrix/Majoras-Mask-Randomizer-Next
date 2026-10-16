//! `WritePlayerModel`: tunic colour support and the playable character
//! (with `PlayerModelUtils.ApplyAdultLinkPatches`).

use crate::{BuildError, Builder};

fn model(name: &str) -> &'static [u8] {
    crate::resources::models(name).unwrap_or_else(|| panic!("missing model {name}"))
}

impl Builder<'_> {
    pub(crate) fn write_player_model(&mut self) -> Result<(), BuildError> {
        // Let tunics use the environment colour, so they can be recoloured.
        for name in [
            "envcolour_fdlink_mod",
            "envcolour_goronlink_mod",
            "envcolour_zoralink_mod",
            "gameplaykeep_fincolors",
            "envcolour_dekulink_mod",
            "gameplay_keep_swordenvs",
        ] {
            self.apply_mod("models", name)?;
        }
        let character = self.plan.choice("Character").to_string();
        match character.as_str() {
            "LinkMM" | "" => self.apply_mod("models", "envcolour_humanlink_mod"),
            "AdultLink" => self.apply_adult_link_patches(),
            _ => {
                // Characters 1-3 (OoT Link, Kafei...) replace the player object.
                let index = self.plan.enum_value("Character") as usize;
                self.apply_mod("mods", ["fix_link_1", "fix_link_2", "fix_link_3"][index - 1])?;
                self.insert_object(model(["link_1", "link_2", "link_3"][index - 1]), 0x11)?;
                if character == "Kafei" {
                    self.insert_object(model("kafei"), 0x1C)?;
                    self.apply_mod("mods", "fix_kafei")?;
                    self.insert_object(model("link_mask"), 0x1FF)?;
                    self.apply_mod("mods", "update_kafei_mask_icon")?;
                    self.insert_object(model("gi_link_mask"), 0x258)?;
                }
                Ok(())
            }
        }
    }

    fn apply_adult_link_patches(&mut self) -> Result<(), BuildError> {
        let r = &mut self.rom;
        r.write_u32(0xC56350, 0x060122C4)?;
        // Boots data.
        for (a, v) in [(0xC5637E, 0x02BC), (0xC56380, 0x0226), (0xC56382, 0x010E), (0xC56384, 0x02BC), (0xC56386, 0x012C), (0xC5638C, 0x0258), (0xC56392, 0x024E), (0xC56398, 0x00C8), (0xC5639A, 0x0082)] {
            r.write_u16(a, v)?;
        }
        r.write(0xC5653C, model("adult_link_code_segaddrs"))?;
        // Sword hitboxes.
        for (a, v) in [(0xC572BC, 0x457A0000u32), (0xC572C0, 0x457A0000), (0xC572C4, 0x45ABE000), (0xC572C8, 0x45ABE000)] {
            r.write_u32(a, v)?;
        }
        r.write_u32(0xC572D4, 0x0601BE60)?;
        r.write_u32(0xC72FA8, 0x800B7058)?;
        self.write_code_u32(0x8019_F854, 0)?;
        let r = &mut self.rom;
        // player_actor floats.
        for (i, v) in [
            0x42600000u32, 0x42B40000, 0x3F800000, 0x42DE0000, 0x428C0000, 0x429ECCCD, 0x426C0000, 0x42240000, 0x41980000, 0x42100000, 0x42480000, 0x42600000, 0x42880000,
            0x428C0000, 0x41900000, 0x41B80000, 0x428C0000,
        ]
        .into_iter()
        .enumerate()
        {
            r.write_u32(0xCD6218 + i as u32 * 4, v)?;
        }
        r.write(0xCD625C, model("adult_link_player_actor_data_1"))?;
        r.write(0xCD62B8, model("adult_link_player_actor_data_2"))?;
        r.write_u32(0xD3BC50, 0x25089D60)?; // Hookshot
        r.write_u32(0xFFA188, 0x3C0142C0)?; // pushing Mikau
        // gameplay_keep vertices and display lists.
        r.write(0x10B0510, model("adult_link_gameplay_keep_vtx_1"))?;
        r.write(0x10B0A90, model("adult_link_gameplay_keep_vtx_2"))?;
        r.write(0x10B1810, model("adult_link_gameplay_keep_vtx_3"))?;
        r.write(0x10B18F0, &0xDE0000000405A2E0u64.to_be_bytes())?;
        r.write(0x10E52A0, model("adult_link_gameplay_keep_vtx_4"))?;
        r.write(0x10E52E0, &0xE700000000000000u64.to_be_bytes())?;
        r.write(0x10E52E8, &0xDA3800010405A2A0u64.to_be_bytes())?;
        r.write(0x10E52F0, &0xDF00000000000000u64.to_be_bytes())?;
        // Masks' field models.
        for (a, v) in [
            (0x11B14A8, 0xDE0000000405A2E0u64),
            (0x11B2620, 0xDE0000000405A2E0),
            (0x11B2780, 0xDE0000000405A2E8),
            (0x11B27B0, 0xDE0000000405A2E0),
            (0x11B2890, 0xDE0000000405A2E8),
            (0x11B28C0, 0xDE0000000405A2E0),
            (0x11D6718, 0xDE0000000405A2E0),
        ] {
            r.write(a, &v.to_be_bytes())?;
        }
        r.write(0xCF5950, model("adult_en_horse"))?;
        self.insert_object(model("adult_obj_link"), 0x11)?;
        self.apply_mod("models", "envcolour_adultlink_mod")?;
        self.insert_object(model("adult_obj_epona"), 0x7D)
    }
}
