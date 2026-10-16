//! Text that depends on where items went (the old `Write*Text` methods
//! called from `WriteItems`).

use crate::items;
use crate::message::{message, Header, MessageEntry, Msg, TextBox, RED};
use crate::plan::PlacedItem;
use crate::stages::items::display_name;
use crate::{BuildError, Builder};

/// `MessageUtils.GetArticle`.
pub(crate) fn article(item: &str, indefinite: Option<&str>) -> String {
    let texts = items::shop_text(item);
    if texts.as_ref().is_some_and(|t| t.is_multiple) {
        return String::new();
    }
    if texts.as_ref().is_some_and(|t| t.is_definite) {
        return "the ".into();
    }
    if let Some(a) = indefinite {
        return a.into();
    }
    let starts_with_vowel = items::name(item).and_then(|n| n.chars().next()).is_some_and(|c| "aeiouAEIOU".contains(c));
    if starts_with_vowel { "an " } else { "a " }.into()
}

/// "a/an/the <name>" filled in at runtime, as the old `RuntimeArticle` +
/// red `RuntimeItemName` pattern.
fn article_and_name(it: &mut Msg, io: &PlacedItem, location: &str) {
    let gi = items::get_item_index(location).unwrap_or(0);
    let display = io.mimic.as_ref().map(|m| m.item.as_str()).unwrap_or(&io.item);
    it.runtime_start(5, gi).text(&article(display, None)).runtime_generic_stop();
    it.color(RED, |it| {
        it.runtime_start(3, gi).text(display_name(io)).runtime_generic_stop();
    });
}

impl Builder<'_> {
    pub(crate) fn write_misc_item_text(&mut self, new: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        let moved = |b: &Self, item: &str| b.item_object(item).and_then(|io| io.location.as_deref()) != Some(item);
        let randomized = |b: &Self, item: &str| b.item_object(item).is_some_and(|io| io.is_randomized);
        let at = |b: &Self, location: &str| b.placed_at(location).cloned();

        // Remove "...Suddenly, memories of Princess Zelda come rushing back to you..."
        if moved(self, "ItemOcarina") || moved(self, "SongTime") {
            new.push(message(0x4C, None, |it| {
                it.quick_text(|it| {
                    it.text("You got the ").red("Ocarina of Time").new_line().text("back!");
                });
                it.new_line().pause_text(10);
                it.red("Princess Zelda").text(" gave you this").new_line();
                it.text("precious instrument.").end_text_box();
                it.text("Set it to ").yellow("\u{B2}").text(" and use ").yellow("\u{B0} ").text("and the").new_line();
                it.text("four ").yellow("\u{B2}").text(" Buttons to play it. Press").new_line();
                it.text("\u{B1} to stop.").end_final_text_box();
            }));
        }

        if at(self, "ItemBottleWitch").is_some_and(|io| io.item != "ItemBottleWitch") {
            new.push(message(0x839, None, |it| {
                it.compile_time_wrap_text("If it's just the Skull Kid, then what harm could he possibly do?").end_text_box();
                it.text("Oh!!").new_line();
                it.text("Well, if that's true, then take this").new_line();
                it.text("and find a way to help her...").new_line();
                it.text("This isn't good...").end_final_text_box();
            }));
            new.push(message(0x83A, None, |it| {
                it.play_sound_effect(0x3901).text("Quick! Go help Koume!").end_conversation().end_final_text_box();
            }));
            new.push(message(0x83B, None, |it| {
                it.text("Oh! Did you find a way to help").new_line();
                it.text("Koume?").end_text_box();
                it.text("Don't tell me...").end_text_box();
                it.text("You need more help, do you?").end_text_box();
                it.text(".........").disable_text_skip2().end_final_text_box();
            }));
        }

        let tingle = ["ItemTingleMapTown", "ItemTingleMapWoodfall", "ItemTingleMapSnowhead", "ItemTingleMapRanch", "ItemTingleMapGreatBay", "ItemTingleMapStoneTower"];
        if tingle.iter().any(|m| randomized(self, m)) {
            new.push(message(0x1D02, None, |it| {
                it.compile_time_wrap_text("Now while I stand here waiting for a fairy of my own, I sell items to help out my father.");
                it.disable_text_skip2().end_final_text_box();
            }));
            new.push(message(0x1D03, None, |it| {
                it.play_sound_effect(0x697B).text("Lucky! Lucky!").new_line();
                it.text("You're so lucky to have a fairy! ").new_line();
                it.text("I know! I know! We should be").new_line();
                it.text("friends!").end_text_box();
                it.text("Yes! Yes! In exchange, I will sell").new_line();
                it.text("you items for cheap as a sign of").new_line();
                it.text("my friendship.").disable_text_skip2().end_final_text_box();
            }));
            new.push(message(0x1D04, None, |it| {
                it.text("Will you buy one of Tingle's ").red("items").text("?").disable_text_skip2().end_final_text_box();
            }));
        }

        if at(self, "TradeItemPendant").is_some_and(|io| io.item != "TradeItemPendant") {
            new.push(message(0x2975, None, |it| {
                it.text("Please take this...").end_final_text_box();
            }));
        }

        if at(self, "MaskBlast").is_some_and(|io| io.item != "MaskBlast") {
            new.push(message(0x2A35, None, |it| {
                it.play_sound_effect(0x690C);
                it.compile_time_wrap_text("Yes, I must thank you. It's dangerous, but maybe you can use it.");
                it.end_conversation().end_final_text_box();
            }));
        }

        let curiosity = ["MundaneItemCuriosityShopBlueRupee", "MundaneItemCuriosityShopRedRupee", "MundaneItemCuriosityShopPurpleRupee", "MundaneItemCuriosityShopGoldRupee"];
        if curiosity.iter().any(|m| randomized(self, m)) {
            new.push(message(0x29EF, None, |it| {
                it.text("For that, I'll give you").new_line();
                it.text("something worth \u{DE}.").new_line();
                it.start_green().two_choices().text("I'll take it.").new_line().text("No thanks").end_final_text_box();
            }));
        }

        // Zora Cape jar game.
        let location = "CollectableZoraCapeJarGame1";
        if let Some(io) = at(self, location).filter(|io| io.item != location) {
            new.push(message(0x126F, None, |it| {
                it.runtime_wrap(|it| {
                    it.text("Well, here's ");
                    article_and_name(it, &io, location);
                    it.text(".");
                });
                it.end_text_box();
                it.text("Except...").new_line();
                it.text("Jar replacement costs ").pink("10 Rupees").text(",").new_line();
                it.text("so I'll have to deduct that.").disable_text_skip2().end_final_text_box();
            }));
            new.push(message(0x1270, None, |it| {
                if io.item == "IceTrap" {
                    it.quick_text(|it| {
                        it.text("You are a ").dark_blue("FOOL").text("!");
                    });
                    it.end_conversation().end_final_text_box();
                } else {
                    it.runtime_wrap(|it| {
                        it.text("You get ");
                        article_and_name(it, &io, location);
                        it.text("!");
                    });
                    it.end_conversation().end_final_text_box();
                }
            }));
        }

        // Dampe's bat rupee.
        let location = "CollectableIkanaGraveyardDay2Bats1";
        if let Some(io) = at(self, location).filter(|io| io.item != location) {
            new.push(message(0x1413, None, |it| {
                it.text("Was it you who chased those bats").new_line();
                it.text("away?").end_text_box();
                it.text("That's a big help... ").new_line();
                it.runtime_wrap(|it| {
                    it.text("It isn't much, but here's ");
                    article_and_name(it, &io, location);
                    it.text(" for your trouble. Take it!");
                });
                it.end_final_text_box();
            }));
        }

        // Replace "Razor Sword is now blunt" with a get-item message for the Kokiri Sword.
        new.push(message(0xF9, Some(Header::new(TextBox::Standard2).icon(0x37)), |it| {
            it.text("You got the ").red("Kokiri Sword").text("!").new_line();
            it.text("This is a hidden treasure of").new_line();
            it.text("the Kokiri, but you can borrow it").new_line();
            it.text("for a while.").end_final_text_box();
        }));

        new.push(message(0xC8, None, |it| {
            it.quick_text(|it| {
                it.text("You've been granted ").green("Magic Power").text("!");
            });
            it.new_line().text("Replenish it with ").red("Magic Jars").new_line();
            it.text("and ").red("Potions").text(".").end_final_text_box();
        }));

        let tokens_randomized = self.plan.custom_item_list.iter().any(|i| items::item_category(i) == Some("SkulltulaTokens"));
        let late_randomized = self.plan.items.iter().any(|io| items::value(&io.id).is_some_and(|v| v >= 433) && io.is_randomized);
        if tokens_randomized || late_randomized {
            self.apply_mod("mods", "fix_piece_of_heart_message")?;
        }
        if tokens_randomized {
            self.apply_mod("mods", "fix_skulltula_tokens")?;
            for (id, color, name, article) in [(0x51u16, crate::message::LIGHT_BLUE, "Ocean Gold Skulltula", "an "), (0x52, crate::message::PINK, "Swamp Gold Skulltula", "a ")] {
                new.push(message(id, Some(Header::new(TextBox::FaintBlue).icon(0x52)), |it| {
                    it.quick_text(|it| {
                        it.text("You got ").text(article).color(color, |it| {
                            it.text(name).new_line().text("Spirit");
                        });
                        it.text("!");
                    });
                    it.pause_text(0x0010).text(" This is your ").color(RED, |it| {
                        it.skulltula_count();
                    });
                    it.text(" one!").end_final_text_box();
                }));
            }
        }

        let mut fairies = items::range_from("CollectibleStrayFairyWoodfall1", 60);
        fairies.push("CollectibleStrayFairyClockTown");
        for fairy in fairies {
            if let Some(io) = self.item_object(fairy) {
                if io.is_randomized || io.item != fairy {
                    self.apply_mod("mods", "fix_fairies")?;
                    break;
                }
            }
        }
        Ok(())
    }
}
