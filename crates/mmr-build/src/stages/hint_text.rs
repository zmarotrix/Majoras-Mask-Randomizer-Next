//! Hint text: Oath to Order, boss remains, and the Great Fairies' and
//! Skulltula houses' location hints (`WriteOathHintText`,
//! `WriteRemainsHintText`, `WriteFairyAndSkullHintText`).

use crate::dotnet_random::DotNetRandom;
use crate::items::{self, number_to_words};
use crate::message::{message, Header, MessageEntry, Msg, TextBox, DARK_BLUE, GREEN, LIGHT_BLUE, PINK, RED, WHITE, YELLOW};
use crate::stages::regions::{region_name, region_preposition};
use crate::{BuildError, Builder};

/// `RuntimeStrayFairyLocations`: "<message> in <region>." filled in at
/// runtime for the fairies (or tokens) at `locations` still to be found.
fn stray_fairy_locations(it: &mut Msg, color: u16, text: &str, is_last: bool, region: &str, locations: &[u16]) {
    let mut preposition = region_preposition(region);
    let name = region_name(region);
    if preposition.as_ref().is_some_and(|p| !p.trim().is_empty()) {
        preposition = preposition.map(|p| format!("{p} "));
    }
    it.runtime_stray_fairy_locations_start(locations);
    it.color(color, |it| {
        it.runtime_wrap(|it| {
            it.text(&format!(" {text} ")).text(preposition.as_deref().unwrap_or("")).red_opt(name.as_deref()).text(".");
        });
    });
    if !is_last {
        it.end_text_box();
    }
    it.runtime_generic_stop();
}

impl Builder<'_> {
    pub(crate) fn write_oath_hint_text(&mut self, new: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        let mut oath = Some(self.obj("SongOath")?);
        if !self.plan.flag("AddSongs") {
            // Where Oath is, by which song spot it took.
            oath = match oath.as_ref().map(|o| o.loc().to_string()).as_deref() {
                Some("SongTime") => oath,
                Some("SongHealing") | Some("SongOath") => None,
                Some("SongEpona") => Some(self.obj("ItemPowderKeg")?),
                Some("SongSoaring") | Some("SongSonata") => Some(self.obj("MaskDeku")?),
                Some("SongStorms") => Some(self.obj("MaskCaptainHat")?),
                Some("SongLullaby") | Some("SongLullabyIntro") => Some(self.obj("MaskGoron")?),
                Some("SongNewWaveBossaNova") => Some(self.obj("MaskZora")?),
                Some("SongElegy") => Some(self.obj("UpgradeMirrorShield")?),
                _ => oath,
            };
        }
        let Some(oath) = oath.filter(|o| o.is_randomized) else { return Ok(()) };
        let region = region_name(&self.region_for_direct_hint(oath.loc())).unwrap_or_default();
        new.push(message(0x208B, None, |it| {
            it.start_light_blue().pause_text(10).text("\"");
            let chars: Vec<char> = region.chars().collect();
            for (i, c) in chars.iter().enumerate() {
                it.text(&c.to_string());
                if i == chars.len() - 1 {
                    it.text(".");
                } else if *c != ' ' {
                    it.pause_text(20);
                }
            }
            it.text("\"").new_line().text(" ").new_line().pause_text(10).text("That's what they're saying.").end_final_text_box();
        }));
        Ok(())
    }

    pub(crate) fn write_remains_hint_text(&mut self, new: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        let remains: Vec<&str> = items::range_from("RemainsOdolwa", 4).into_iter().filter(|r| self.item_object(r).is_some_and(|io| io.item == *r)).collect();
        let randomized = remains.iter().any(|r| self.item_object(r).is_some_and(|io| io.is_randomized))
            && !self.plan.has("BossRemainsMode", "GreatFairyRewards")
            && !self.plan.has("BossRemainsMode", "ShuffleOnly")
            && !self.plan.has("BossRemainsMode", "KeepWithinTemples");
        if !(randomized || (!remains.is_empty() && remains.len() < 4)) {
            return Ok(());
        }
        // Shuffled with .NET's Random seeded by the seed, as the old code did.
        let mut random = DotNetRandom::new(self.plan.seed as i32);
        let mut keyed: Vec<(i32, &str)> = remains.iter().map(|r| (random.next(), *r)).collect();
        keyed.sort_by_key(|(k, _)| *k);
        let mut regions: Vec<String> = Vec::new();
        for (_, remain) in keyed {
            let location = self.obj(remain)?.loc().to_string();
            let region = if randomized {
                region_name(&self.region_for_direct_hint(&location)).unwrap_or_default()
            } else {
                self.region_area_of_temple(&location).unwrap_or_default()
            };
            if !regions.contains(&region) {
                regions.push(region);
            }
        }
        let count = number_to_words(remains.len() as i64);
        let is_are = if remains.len() == 1 { "is" } else { "are" };
        new.push(message(0x200B, None, |it| {
            it.start_pink().play_sound_effect(0x6851).compile_time_wrap(|w| {
                for r in &regions {
                    w.text(r).text(". ").pause_text(10);
                }
                w.text("Hurry...").red(&format!("The {count}")).text(&format!(" who {is_are} there... Bring them ")).red("here").text("...");
            });
            it.end_final_text_box();
        }));
        new.push(message(0x216, None, |it| {
            it.start_light_blue().text("That mask...").new_line();
            it.pause_text(40).text("The Skull Kid uses the power of").new_line();
            it.text("that mask to do those terrible").new_line();
            it.text("things.").end_text_box();
            it.text("Well...whatever it takes, we've").new_line();
            it.text("gotta do something about it.").end_text_box();
            it.compile_time_wrap(|w| {
                w.text("...The ");
                for (i, r) in regions.iter().enumerate() {
                    let r = if randomized { r.clone() } else { r.to_lowercase() };
                    w.red(&r);
                    if i + 2 < regions.len() {
                        w.text(", ");
                    } else if i + 1 < regions.len() {
                        w.text(" and ");
                    }
                }
                w.text(" that Tael was trying to tell us about...");
            });
            it.end_text_box();
            it.compile_time_wrap_text("I have no idea what he was talking about...");
            it.end_text_box();
            it.text("And what do you suppose he").new_line();
            it.text("meant by \"").red_with(|it| {
                it.text(&format!("the {count} who {is_are}")).new_line().text("there");
            });
            it.text("?\"").end_text_box();
            it.text("I have no idea. He always").new_line();
            it.text("skips important stuff. I guess we").new_line();
            it.text("should just go and find out...").end_final_text_box();
        }));
        self.apply_mod("mods", "tatl_remains_hint")
    }

    pub(crate) fn write_fairy_and_skull_hint_text(&mut self, new: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        let town = self.obj("CollectibleStrayFairyClockTown")?;
        if town.loc() != "CollectibleStrayFairyClockTown" {
            let region = self.region_for_direct_hint(town.loc());
            let mut preposition = region_preposition(&region);
            let name = preposition.as_ref().and_then(|_| region_name(&region));
            if preposition.as_ref().is_some_and(|p| !p.trim().is_empty()) {
                preposition = preposition.map(|p| format!("{p} "));
            }
            let prep = preposition.clone().unwrap_or_default();
            new.push(message(0x578, None, |it| {
                it.play_sound_effect(0x6938).start_light_blue().text("Young one! Please hear my plea!").new_line();
                it.text("I have been broken and shattered").new_line();
                it.text("to pieces by the masked Skull Kid.").end_text_box();
                it.compile_time_wrap(|w| {
                    w.text("Please, find the").red(" one ").text("Stray Fairy lost ");
                    w.text(&prep).red_opt(name.as_deref()).text(", and bring her ");
                    w.text("to this ").red("Fairy Fountain").text(".");
                });
                it.end_final_text_box();
            }));
            new.push(message(0x580, None, |it| {
                it.play_sound_effect(0x6938).start_light_blue().text("You...kind young one.").end_text_box();
                it.compile_time_wrap(|w| {
                    w.text("Please, find the").red(" one ").text("Stray Fairy who's lost ");
                    w.text(&prep).red(name.as_deref().unwrap_or("somewhere")).text(" and bring her ");
                    w.text("back to this ").red("Fairy's Fountain").text(".");
                });
                it.end_final_text_box();
            }));
        }

        // Fairies and tokens still in place, by the region they belong to,
        // then by the region they are now in.
        let mut sources: Vec<&str> = items::range_from("CollectibleStrayFairyWoodfall1", 60);
        sources.extend(items::range_from("CollectibleSwampSpiderToken1", 30));
        sources.extend(items::range_from("CollectibleOceanSpiderToken1", 30));
        let mut groups: Vec<(String, Vec<(String, Vec<String>)>)> = Vec::new();
        for fairy in sources {
            let Some(io) = self.item_object(fairy).filter(|io| io.item == fairy) else { continue };
            let home = self.region_of(fairy).unwrap_or_default();
            let location = io.loc().to_string();
            let spots: Vec<String> = match items::table().attr(&location, "MultiLocation").and_then(|a| a.get("Locations").as_array().cloned()) {
                Some(list) => list.iter().filter_map(|v| v.as_str()).map(|l| self.region_of(l).unwrap_or_default()).collect(),
                None => vec![self.region_of(&location).unwrap_or_default()],
            };
            let gi = match groups.iter().position(|(g, _)| *g == home) {
                Some(i) => i,
                None => {
                    groups.push((home, Vec::new()));
                    groups.len() - 1
                }
            };
            for region in spots.into_iter().filter(|r| r != "Misc") {
                let inner = &mut groups[gi].1;
                match inner.iter_mut().find(|(r, _)| *r == region) {
                    Some((_, l)) => l.push(location.clone()),
                    None => inner.push((region, vec![location.clone()])),
                }
            }
        }
        // Groups whose fairies are all at Misc locations are kept, empty.
        if !groups.is_empty() {
            self.apply_mod("mods", "dungeon_fairy_npc_hint")?;
        }
        let group = |name: &str| groups.iter().find(|(g, _)| g == name).map(|(_, l)| l.clone());
        let gis = |locations: &[String]| -> Vec<u16> { locations.iter().map(|l| items::get_item_index(l).unwrap_or(0)).collect() };

        // The four temple Great Fairies.
        let temples: [(&str, [u16; 3], fn(&mut Msg), fn(&mut Msg), fn(&mut Msg)); 4] = [
            (
                "WoodfallTemple",
                [0x582, 0x583, 0x584],
                |it| {
                    it.text("Kind young one! Please hear my").new_line();
                    it.text("plea! Please find the fairies").new_line();
                    it.text("who match our ").red("color").text(".").end_text_box();
                    it.text("Please bring them back to us!").end_final_text_box();
                },
                |it| {
                    it.text("Please fine a way to save the").new_line();
                    it.text("fairies, and bring them back").new_line();
                    it.text("here!").end_final_text_box();
                },
                |it| {
                    it.text("Please save the fairies so I can").new_line();
                    it.text("be returned to my former shape!").end_final_text_box();
                },
            ),
            (
                "SnowheadTemple",
                [0x585, 0x586, 0x587],
                |it| {
                    it.text("Oh, kind, young one!").new_line();
                    it.text("Please hear our plea! Please save").new_line();
                    it.text("the ").color_text(GREEN, "fairies ").text("who match our ").color_text(GREEN, "color").text(" and").new_line();
                    it.text("bring them back to us!").end_final_text_box();
                },
                |it| {
                    it.text("Please find a way to save the").new_line();
                    it.text("fairies and bring them back").new_line();
                    it.text("here!").end_final_text_box();
                },
                |it| {
                    it.text("Please bring them back here so").new_line();
                    it.text("I can be returned to my former").new_line();
                    it.text("shape!").end_final_text_box();
                },
            ),
            (
                "GreatBayTemple",
                [0x588, 0x589, 0x58A],
                |it| {
                    it.text("Oh, kind young one!").new_line();
                    it.text("Please find the fairies who are").new_line();
                    it.text("the same ").color_text(DARK_BLUE, "color").text(" as we are and");
                    it.text("bring them back to us!").end_final_text_box();
                },
                |it| {
                    it.text("Please find a way to save the").new_line();
                    it.text("fairies, and bring them back").new_line();
                    it.text("here!").end_final_text_box();
                },
                |it| {
                    it.text("Please save them and bring them").new_line();
                    it.text("back here!").end_final_text_box();
                },
            ),
            (
                "StoneTowerTemple",
                [0x58B, 0x58C, 0x58D],
                |it| {
                    it.text("Oh, kind young one!").new_line();
                    it.text("Please hear our plea! Please find").new_line();
                    it.text("the fairies who are the same").new_line();
                    it.color_text(YELLOW, "color").text(" as we are and bring them").new_line();
                    it.text("back to us!").end_final_text_box();
                },
                |it| {
                    it.text("Please save the the fairies and").new_line();
                    it.text("bring them back here!").end_final_text_box();
                },
                |it| {
                    it.text("Please save them and bring them").new_line();
                    it.text("back here so I can be returned").new_line();
                    it.text("to my former shape!").end_final_text_box();
                },
            ),
        ];
        for (region, ids, plea, again, outro) in temples {
            let Some(list) = group(region) else { continue };
            new.push(message(ids[0], None, |it| {
                it.play_sound_effect(0x6938).start_light_blue();
                plea(it);
            }));
            new.push(message(ids[1], None, |it| {
                it.play_sound_effect(0x6938).start_light_blue();
                again(it);
            }));
            new.push(
                message(ids[2], Some(Header::new(TextBox::Standard).icon(0x11)), |it| {
                    it.play_sound_effect(0x6938).start_light_blue().text("There should still be...").end_text_box();
                    for (r, locations) in &list {
                        stray_fairy_locations(it, LIGHT_BLUE, "trapped", false, r, &gis(locations));
                    }
                    outro(it);
                })
                .exclude_quick_text()
                .transfer(),
            );
        }

        let any_randomized = |b: &Self, first: &str| items::range_from(first, 30).iter().any(|t| b.item_object(t).is_some_and(|io| io.is_randomized));
        if let Some(list) = group("SwampSpiderHouseItems").filter(|_| any_randomized(self, "CollectibleSwampSpiderToken1")) {
            new.push(message(0x911, None, |it| {
                it.compile_time_wrap(|w| {
                    w.text("I beg of you...To lift the curse...Find them all...The ").red("golden spider tokens").text("...");
                });
                it.disable_text_skip2().end_final_text_box();
            }));
            let half = list.len() / 2;
            let first_half = |it: &mut Msg| {
                for (i, (r, locations)) in list[..half].iter().enumerate() {
                    stray_fairy_locations(it, WHITE, "hiding", i == half - 1, r, &gis(locations));
                }
            };
            new.push(
                message(0x912, Some(Header::new(TextBox::Standard).y(0).icon(0x52).next_message(0x9912)), |it| {
                    it.text("There should still be...").end_text_box();
                    first_half(it);
                    it.disable_text_skip().end_final_text_box();
                })
                .exclude_quick_text()
                .transfer(),
            );
            self.extra_messages.push(
                message(0x9912, Some(Header::new(TextBox::Standard).y(0).icon(0x52)), |it| {
                    for (r, locations) in &list[half..] {
                        stray_fairy_locations(it, WHITE, "hiding", false, r, &gis(locations));
                    }
                    it.text("Please hurry...").disable_text_skip2().end_final_text_box();
                })
                .exclude_quick_text(),
            );
            new.push(
                message(0x914, Some(Header::new(TextBox::Standard).y(0).icon(0x52).next_message(0x9912)), |it| {
                    it.text("Please... There should still be...").end_text_box();
                    first_half(it);
                    it.disable_text_skip().end_final_text_box();
                })
                .exclude_quick_text()
                .transfer(),
            );
        }
        if let Some(list) = group("OceanSpiderHouseItems").filter(|_| any_randomized(self, "CollectibleOceanSpiderToken1")) {
            let half = list.len() / 2;
            new.push(
                message(0x1135, Some(Header::new(TextBox::Standard).y(0).icon(0x52).next_message(0xA135)), |it| {
                    it.text("I beg you! Lift the curse on this").new_line();
                    it.text("place! There should still be...").end_text_box();
                    for (i, (r, locations)) in list[..half].iter().enumerate() {
                        stray_fairy_locations(it, WHITE, "hiding", i == half - 1, r, &gis(locations));
                    }
                    it.disable_text_skip().end_final_text_box();
                })
                .exclude_quick_text()
                .transfer(),
            );
            self.extra_messages.push(
                message(0xA135, Some(Header::new(TextBox::Standard).y(0).icon(0x52)), |it| {
                    for (r, locations) in &list[half..] {
                        stray_fairy_locations(it, WHITE, "hiding", false, r, &gis(locations));
                    }
                    it.text("If you lift the curse, I'll buy").new_line();
                    it.text("this place off you! Please hurry...").disable_text_skip2().end_final_text_box();
                })
                .exclude_quick_text()
                .transfer(),
            );
        }
        let _ = (RED, PINK);
        Ok(())
    }
}
