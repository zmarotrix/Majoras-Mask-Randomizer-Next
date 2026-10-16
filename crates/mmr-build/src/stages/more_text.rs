//! More item text: world models, the victory condition, dungeon items, and
//! messages for items that need their own (`WriteWorldModelText`,
//! `WriteVictoryConditionText`, `WriteDungeonItemText`).

use crate::enums;
use crate::items;
use crate::message::{message, units, Header, MessageEntry, Msg, TextBox, GREEN, LIGHT_BLUE, PINK, YELLOW};
use crate::stages::{article, pronoun};
use crate::{BuildError, Builder};

impl Builder<'_> {
    pub(crate) fn write_world_model_text(&mut self, new: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        // Update Moon's Tear messages.
        let moons_tear_item = self.placed("TradeItemMoonTear")?;
        if moons_tear_item.item != "TradeItemMoonTear"
        {
            new.push(message(0x5E3, None, |it|
                {
                    it.text("That is one of the lunar objects").new_line()
                    .text("that has been blazing from the").new_line()
                    .text("surface of the moon lately.")
                    .end_text_box()
                    .compile_time_wrap(|wrapped|
                    {
                        wrapped.text("They fall from what looks to be the moon's eye, I call ")
                        .text(pronoun(moons_tear_item.display()))
                        .text(" ")
                        .text(&article(moons_tear_item.display(), None))
                        .red(moons_tear_item.display_name())
                        .text(".")
                        ;
                    })
                    .end_text_box()
                    .text("They are rare, valued by many").new_line()
                    .text("in town.")
                    .end_final_text_box();
                }));
            new.push(message(0x5ED, None, |it|
                {
                    it.text("That ill-mannered troublemaker").new_line()
                    .text("from the other day said he'd").new_line()
                    .text("break my instruments...")
                    .end_text_box()
                    .compile_time_wrap(|wrapped|
                    {
                        wrapped.text("He said he'd steal my ")
                        .red(moons_tear_item.display_name())
                        .text("... There was no stopping him.")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
            new.push(message(0x5F2, None, |it|
                {
                    it.text("Well, did you find that").new_line()
                    .red("troublemaker").text("? And that loud").new_line()
                    .text("noise...What was that?")
                    .end_text_box()
                    .compile_time_wrap(|wrapped|
                    {
                        wrapped.text("Perhaps another ")
                        .red(moons_tear_item.display_name())
                        .text(" has falled nearby...Go through that door and take a look outside.")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
        }

        // Update Seahorse messages.
        let seahorse_item = self.placed("MundaneItemSeahorse")?;
        if seahorse_item.item != "MundaneItemSeahorse"
        {
            new.push(message(0x106F, None, |it|
                {
                    it.play_sound_effect(0x694C)
                    .text("Are you interested in that?")
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("It's rare, isn't it? It's called ")
                        .runtime_article(seahorse_item.display(), seahorse_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(seahorse_item.display_name(), seahorse_item.loc());
                        })
                        .text(".")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
            new.push(message(0x1074, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("If you want that ")
                        .red_with(|it|
                        {
                            it.runtime_item_name(seahorse_item.display_name(), seahorse_item.loc());
                        })
                        .text(", bring me a ")
                        .red("pictograph")
                        .text(" of a ")
                        .red("female pirate")
                        .text(".")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
        }
        Ok(())
    }

    pub(crate) fn write_victory_condition_text(&mut self, new: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        let table = enums::get("VictoryMode");
        let value = |name: &str| table.get(name).map(|v| v.value).unwrap_or(0);
        let has = |mode: i64, name: &str| mode & value(name) == value(name);
        let mut mode = self.plan.enum_value("VictoryMode");
        if has(mode, "FourBossRemains") {
            mode &= !(value("OneBossRemains") | value("TwoBossRemains") | value("ThreeBossRemains"));
        } else if has(mode, "ThreeBossRemains") {
            mode &= !(value("OneBossRemains") | value("TwoBossRemains"));
        } else if has(mode, "TwoBossRemains") {
            mode &= !value("OneBossRemains");
        }
        // `Enum.GetValues` order, i.e. by value.
        let mut values: Vec<_> = table.values.iter().collect();
        values.sort_by_key(|v| v.value as u64);
        let mut flavor: Vec<String> = Vec::new();
        for v in values {
            if mode & v.value == v.value {
                if let Some(a) = v.attributes.iter().find(|a| a.kind == "VictoryModeFlavorText") {
                    flavor.push(a.get("Text").as_str().unwrap_or("").to_string());
                }
            }
        }
        if flavor.is_empty() {
            return Ok(());
        }
        let mut flavor2 = flavor.clone();
        if !has(mode, "DirectToCredits") {
            flavor.push("get [RED]Majora's Mask[WHITE] back".into());
            flavor2.push("recover [RED]Majora's Mask[WHITE]".into());
        }
        let past: Vec<String> = flavor2
            .iter()
            .map(|t| t.replace("recover", "recovered").replace("gather", "gathered").replace("collect", "collected").replace("maximize", "maximized").replace("find", "found"))
            .collect();
        // "a, b and c", with [RED]/[WHITE] as colour codes.
        fn list(w: &mut Msg, texts: &[String]) {
            for (i, t) in texts.iter().enumerate() {
                let t = t.replace("[RED]", "\u{1}").replace("[WHITE]", "\u{0}");
                w.text(&t);
                if i + 2 < texts.len() {
                    w.text(", ");
                } else if i + 1 < texts.len() {
                    w.text(" and ");
                }
            }
        }
        new.push(message(0x1FC9, None, |it| {
            it.compile_time_wrap(|w| {
                w.text("Were you able to ");
                list(w, &flavor);
                w.text("?");
            });
            it.disable_text_skip2().end_final_text_box();
        }));
        new.push(message(0x1FCA, None, |it| {
            it.quick_text(|it| {
                it.text("You still haven't done it!");
            });
            it.end_text_box();
            it.text("I keep telling you that if you don't").new_line();
            it.text("get it done soon, terrible things").new_line();
            it.text("will happen!!!").disable_text_skip2().end_final_text_box();
        }));
        new.push(message(0x1FCD, None, |it| {
            it.compile_time_wrap(|w| {
                w.text("You'll do fine. Surely you will be able to ");
                list(w, &flavor2);
                w.text(".");
            });
            it.end_text_box();
            it.text("Only ").red("\u{E7} ").text("remain.").new_line();
            it.text("But time is not eternal.").new_line();
            it.text("Please make the most of your").new_line();
            it.text("time.").end_text_box();
            it.text("I believe in you.").new_line();
            it.text("I will be waiting here for you.").new_line();
            it.play_sound_effect(0x697F).text("Ho, ho, ho.").end_final_text_box();
        }));
        new.push(message(0x1FCF, None, |it| {
            it.compile_time_wrap(|w| {
                w.text("So then...").new_line().text("Have you ");
                list(w, &past);
                w.text("?");
            });
            it.disable_text_skip2().end_final_text_box();
        }));
        new.push(message(0x2006, None, |it| {
            it.compile_time_wrap(|w| {
                w.text("You'll be fine. Surely you should be able to ");
                list(w, &flavor2);
                w.text(".");
            });
            it.end_text_box();
            it.text("Yet there's no time left.").end_text_box();
            it.text("But time is not eternal.").new_line();
            it.text("Please value your time.").new_line();
            it.text("I believe in you. I'll be").new_line();
            it.text("waiting here for you.").end_text_box();
            it.text("Ho, ho, ho.").end_final_text_box();
        }));
        Ok(())
    }

    /// Items whose get-item message lives in the extended table.
    pub(crate) fn write_custom_item_messages(&mut self) {
        for item in ["CollectableIkanaGraveyardDay2Bats1", "FrogWoodfallTemple", "FrogGreatBayTemple", "FrogSwamp", "FrogLaundryPool"] {
            if let Some((id, text)) = items::exclusive_message(item) {
                self.extra_messages.push(MessageEntry::new(id, units(text)));
            }
        }
    }

    pub(crate) fn write_dungeon_item_text(&mut self, new: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        const MESSAGE_IDS: [u16; 20] = [0x3C, 0x3D, 0x3E, 0x3F, 0x74, 0x40, 0x4D, 0x4E, 0x53, 0x75, 0x54, 0x61, 0x64, 0x6E, 0x76, 0x70, 0x71, 0x72, 0x73, 0x77];
        let dungeons = [(PINK, "Woodfall Temple"), (GREEN, "Snowhead Temple"), (LIGHT_BLUE, "Great Bay Temple"), (YELLOW, "Stone Tower Temple")];
        let locations: Vec<Option<String>> = ["AreaWoodFallTempleAccess", "AreaSnowheadTempleAccess", "AreaGreatBayTempleAccess", "AreaInvertedStoneTowerTempleAccess"]
            .iter()
            .map(|i| self.item_object(i).and_then(|io| io.location.as_deref()).and_then(items::entrance_name).map(String::from))
            .collect();
        let bosses: Vec<Option<String>> = ["AreaOdolwasLair", "AreaGohtsLair", "AreaGyorgsLair", "AreaTwinmoldsLair"]
            .iter()
            .map(|l| self.placed_at(l).and_then(|io| items::entrance_name(&io.item)).map(String::from))
            .collect();
        const NAMES: [(&str, &str); 5] = [("a ", "Small Key"), ("the ", "Boss Key"), ("the ", "Dungeon Map"), ("the ", "Compass"), ("a ", "Stray Fairy")];
        const ICONS: [u8; 5] = [0x3C, 0x3D, 0x3E, 0x3F, 0x11];
        let map_reveals = self.plan.has("DungeonNavigationMode", "MapRevealsLocation");
        let compass_reveals = self.plan.has("DungeonNavigationMode", "CompassRevealsBoss");
        for (i, &id) in MESSAGE_IDS.iter().enumerate() {
            let (kind, dungeon) = (i % 5, i / 5);
            let (color, dungeon_name) = dungeons[dungeon];
            new.push(message(id, Some(Header::new(TextBox::FaintBlue).icon(ICONS[kind])), |it| {
                it.quick_text(|it| {
                    it.text("You found ").text(NAMES[kind].0).red(NAMES[kind].1).text(" for").new_line();
                    it.color_text(color, dungeon_name).text("!");
                });
                if kind == 2 && map_reveals && locations[dungeon].is_some() {
                    it.pause_text(0x10).new_line().compile_time_wrap(|w| {
                        w.text("The entrance is at ").red(locations[dungeon].as_deref().unwrap()).text(".");
                    });
                } else if kind == 3 && compass_reveals && bosses[dungeon].is_some() {
                    it.pause_text(0x10).new_line().compile_time_wrap(|w| {
                        w.text("It points at ").red(bosses[dungeon].as_deref().unwrap()).text(".");
                    });
                } else if kind == 4 {
                    it.pause_text(0x10).new_line().text("This is your ").red_with(|it| {
                        it.stray_fairy_count();
                    });
                    it.text(" one!");
                }
                it.end_final_text_box();
            }));
        }
        Ok(())
    }
}
