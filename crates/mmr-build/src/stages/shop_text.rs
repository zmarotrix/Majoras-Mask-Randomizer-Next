//! Shop and salesperson text (`WriteShopText`): what each seller offers
//! now that items moved.

use crate::items;
use crate::message::{message, MessageEntry};
use crate::plan::PlacedItem;
use crate::{BuildError, Builder};

impl Builder<'_> {
    /// The item placed at a location (`ItemList.First(io => io.NewLocation == location)`).
    pub(crate) fn placed(&self, location: &str) -> Result<PlacedItem, BuildError> {
        self.placed_at(location).cloned().ok_or_else(|| BuildError::Data(format!("nothing placed at {location}")))
    }

    pub(crate) fn write_shop_text(&mut self, new: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        // Tingle's maps and the Milk Bar: two items per menu.
        const MENUS: [(u16, bool, &str, &str); 7] = [
            (0x1D11, true, "ItemTingleMapTown", "ItemTingleMapWoodfall"),
            (0x1D12, true, "ItemTingleMapWoodfall", "ItemTingleMapSnowhead"),
            (0x1D13, true, "ItemTingleMapSnowhead", "ItemTingleMapRanch"),
            (0x1D14, true, "ItemTingleMapRanch", "ItemTingleMapGreatBay"),
            (0x1D15, true, "ItemTingleMapGreatBay", "ItemTingleMapStoneTower"),
            (0x1D16, true, "ItemTingleMapStoneTower", "ItemTingleMapTown"),
            (0x2B0B, false, "ShopItemMilkBarMilk", "ShopItemMilkBarChateau"),
        ];
        for (message_id, tingle, location1, location2) in MENUS {
            let (item1, item2) = (self.placed(location1)?, self.placed(location2)?);
            let header = self.messages.get(message_id).and_then(|m| m.header).ok_or_else(|| BuildError::Data(format!("missing message {message_id:#X}")))?;
            let costs = [u16::from_be_bytes([header[5], header[6]]), u16::from_be_bytes([header[7], header[8]])];
            let lines: Vec<(String, String, &str)> = [(&item1, costs[0]), (&item2, costs[1])]
                .iter()
                .map(|(io, cost)| {
                    let cost_text = format!("{cost} Rupee{}", if *cost != 1 { "s" } else { "" });
                    (shortened(io.display_name(), 35 - format!(": {cost_text}").len()), cost_text, io.loc())
                })
                .collect();
            new.push(message(message_id, None, |it| {
                if !tingle {
                    it.text("What'll it be?").end_text_box();
                }
                it.start_green_text().three_choices();
                for (name, cost, location) in &lines {
                    it.runtime_item_name(name, location).text(": ");
                    if tingle {
                        it.red(cost);
                    } else {
                        it.pink(cost);
                    }
                    it.new_line();
                }
                it.text(if tingle { "No Thanks" } else { "Nothing" }).end_final_text_box();
            }));
        }

        // update business scrub
        let business_scrub_item = self.placed("HeartPieceTerminaBusinessScrub")?;
        new.push(message(0x1631, None, |it|
            {
                it.play_sound_effect(0x3AD2)
                .runtime_wrap(|it|
                {
                    it.text("Please! I'll sell you ")
                    .runtime_article(business_scrub_item.display(), business_scrub_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(business_scrub_item.display_name(), business_scrub_item.loc());
                    })
                    .text(" if you just keep this place a secret...")
                    ;
                })
                .disable_text_skip2()
                .end_final_text_box();
            }));
        new.push(message(0x1632, None, |it|
            {
                it.pink("150 Rupees").text(" for").runtime_pronoun_or_amount(business_scrub_item.display(), business_scrub_item.loc()).text("!").new_line()
                .text(" ").new_line()
                .start_green_text()
                .two_choices()
                .text("I'll buy ").runtime_pronoun(business_scrub_item.display(), business_scrub_item.loc()).new_line()
                .text("No thanks")
                .end_final_text_box();
            }));
        new.push(message(0x1634, None, |it|
            {
                it.text("What about for ").pink("100 Rupees").text("?").new_line()
                .text(" ").new_line()
                .start_green_text()
                .two_choices()
                .text("I'll buy ").runtime_pronoun(business_scrub_item.display(), business_scrub_item.loc()).new_line()
                .text("No thanks")
                .end_final_text_box();
            }));

        // update biggest bomb bag purchase
        let biggest_bomb_bag_item = self.placed("UpgradeBiggestBombBag")?;
        new.push(message(0x15F5, None, |it|
            {
                it.runtime_wrap(|it|
                {
                    it.text("I sell ")
                    .runtime_article(biggest_bomb_bag_item.display(), biggest_bomb_bag_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(&biggest_bomb_bag_item.alternate_name(), biggest_bomb_bag_item.loc());
                    })
                    .text(", but I'm focusing my marketing efforts on ").red("Gorons").text(".")
                    ;
                })
                .end_text_box()
                .compile_time_wrap_text("What I'd really like to do is go back home and do business where I'm surrounded by trees and grass.")
                .disable_text_skip2()
                .end_final_text_box();
            }));
        new.push(message(0x15FF, None, |it|
            {
                it.play_sound_effect(0x398C)
                .text("Right now, I've got a ").red("special").new_line()
                .text("offer just for you.")
                .disable_text_skip2()
                .end_final_text_box();
            }));
        new.push(message(0x1600, None, |it|
            {
                it.play_sound_effect(0x3881)
                .runtime_wrap(|it|
                {
                    it.text("I'll give you ")
                    .runtime_article(biggest_bomb_bag_item.display(), biggest_bomb_bag_item.loc(), Some("my "))
                    .red_with(|it|
                    {
                        it.runtime_item_name(biggest_bomb_bag_item.display_name(), biggest_bomb_bag_item.loc());
                    })
                    .text(", regularly priced at ")
                    .pink("1000 Rupees")
                    .text("...")
                    ;
                })
                .end_text_box()
                .text("In return, you'll give me just").new_line()
                .pink("200 Rupees").text("!")
                .disable_text_skip2()
                .end_final_text_box();
            }));
        new.push(message(0x1606, None, |it|
            {
                it.play_sound_effect(0x3881)
                .runtime_wrap(|it|
                {
                    it.text("I'll give you ")
                    .runtime_article(biggest_bomb_bag_item.display(), biggest_bomb_bag_item.loc(), Some("my "))
                    .red_with(|it|
                    {
                        it.runtime_item_name(biggest_bomb_bag_item.display_name(), biggest_bomb_bag_item.loc());
                    })
                    .text(", regularly priced at ")
                    .pink("1000 Rupees")
                    .text(", for just ")
                    .pink("200 Rupees")
                    .text("!")
                    ;
                })
                .disable_text_skip2()
                .end_final_text_box();
            }));

        // update swamp scrub purchase
        let magic_bean_item = self.placed("ShopItemBusinessScrubMagicBean")?;
        new.push(message(0x15E1, None, |it|
            {
                it.play_sound_effect(0x39A7)
                .runtime_wrap(|it|
                {
                    it.text("I'm selling ")
                    .runtime_article(magic_bean_item.display(), magic_bean_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(magic_bean_item.display_name(), magic_bean_item.loc());
                    })
                    .text(" to Deku Scrubs, but I'll really like to leave my hometown.")
                    ;
                })
                .end_text_box()
                .compile_time_wrap_text("I'm hoping to find some success in a livelier place!")
                .disable_text_skip2()
                .end_final_text_box();
            }));

        new.push(message(0x15E9, None, |it|
            {
                it.play_sound_effect(0x3AD2)
                .runtime_wrap(|it|
                {
                    it.text("Do you know what ")
                    .runtime_article(magic_bean_item.display(), magic_bean_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(&magic_bean_item.alternate_name(), magic_bean_item.loc());
                    })
                    .text(" ")
                    .runtime_verb(magic_bean_item.display(), magic_bean_item.loc())
                    .text(", sir?")
                    ;
                })
                .new_line()
                .text("I'll sell you").runtime_pronoun_or_amount(magic_bean_item.display(), magic_bean_item.loc()).text(" for ").pink("10 Rupees").text(".")
                .disable_text_skip2()
                .end_final_text_box();
            }));

        new.push(message(0x15F3, None, |it|
            {
                it.play_sound_effect(0x3AD2)
                .runtime_wrap(|it|
                {
                    it.text("Do you know what ")
                    .runtime_article(magic_bean_item.display(), magic_bean_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(&magic_bean_item.alternate_name(), magic_bean_item.loc());
                    })
                    .text(" ")
                    .runtime_verb(magic_bean_item.display(), magic_bean_item.loc())
                    .text("?")
                    ;
                })
                .new_line()
                .text("I'll sell you").runtime_pronoun_or_amount(magic_bean_item.display(), magic_bean_item.loc()).text(" for ").pink("10 Rupees").text(".")
                .disable_text_skip2()
                .end_final_text_box();
            }));

        // update ocean scrub purchase
        let green_potion_item = self.placed("ShopItemBusinessScrubGreenPotion")?;
        new.push(message(0x1608, None, |it|
            {
                it.play_sound_effect(0x39A7)
                .runtime_wrap(|it|
                {
                    it.text("I'm selling ")
                    .runtime_article(green_potion_item.display(), green_potion_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(&green_potion_item.alternate_name(), green_potion_item.loc());
                    })
                    .text(", but I'm focusing my marketing efforts on Zoras.")
                    ;
                })
                .end_text_box()
                .compile_time_wrap_text("Actually, I'd like to do business someplace where it's cooler and the air is clean.")
                .disable_text_skip2()
                .end_final_text_box();
            }));

        new.push(message(0x1612, None, |it|
            {
                it.play_sound_effect(0x398C)
                .runtime_wrap(|it|
                {
                    it.text("I'll sell you ")
                    .runtime_article(green_potion_item.display(), green_potion_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(green_potion_item.display_name(), green_potion_item.loc());
                    })
                    .text(" for ")
                    .pink("40 Rupees")
                    .text("!")
                    ;
                })
                .end_conversation()
                .end_final_text_box();
            }));

        // "Green Potion" -> "G-Green P-Potion".
        let cold_item_name: String = items::name(green_potion_item.display())
            .unwrap_or("")
            .chars()
            .flat_map(|c| if c.is_ascii_uppercase() { vec![c, '-', c] } else { vec![c] })
            .collect();
        new.push(message(0x1617, None, |it|
            {
                it.play_sound_effect(0x398C)
                .runtime_wrap(|it|
                {
                    it.text("I'll s-sell you ")
                    .runtime_article(green_potion_item.display(), green_potion_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(&cold_item_name, green_potion_item.loc());
                    })
                    .text(" for ")
                    .pink("40 Rupees")
                    .text(".")
                    ;
                })
                .end_conversation()
                .end_final_text_box();
            }));

        new.push(message(0x1618, None, |it|
            {
                it.text("D-Do we have a deal?").new_line()
                .text(" ").new_line()
                .start_green_text()
                .two_choices()
                .text("Yes").new_line()
                .text("No")
                .end_final_text_box();
            }));

        // update canyon scrub purchase
        let blue_potion_item = self.placed("ShopItemBusinessScrubBluePotion")?;
        new.push(message(0x161C, None, |it|
            {
                it.play_sound_effect(0x39A7)
                .runtime_wrap(|it|
                {
                    it.text("I'm here to sell ")
                    .runtime_article(blue_potion_item.display(), blue_potion_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(&blue_potion_item.alternate_name(), blue_potion_item.loc());
                    })
                    .text(".")
                    ;
                })
                .end_text_box()
                .compile_time_wrap_text("Actually, I want to do business in the sea breeze while listening to the sound of the waves.")
                .disable_text_skip2()
                .end_final_text_box();
            }));

        new.push(message(0x1626, None, |it|
            {
                it.play_sound_effect(0x3AD2)
                .runtime_wrap(|it|
                {
                    it.text("Don't you need ")
                    .runtime_article(blue_potion_item.display(), blue_potion_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(&blue_potion_item.alternate_name(), blue_potion_item.loc());
                    })
                    .text("? I'll sell you")
                    .runtime_pronoun_or_amount(blue_potion_item.display(), blue_potion_item.loc())
                    .text(" for ")
                    .pink("100 Rupees")
                    .text(".")
                    ;
                })
                .disable_text_skip2()
                .end_final_text_box();
            }));

        new.push(message(0x162D, None, |it|
            {
                it.play_sound_effect(0x398C)
                .runtime_wrap(|it|
                {
                    it.text("I'll sell you ")
                    .runtime_article(blue_potion_item.display(), blue_potion_item.loc(), None)
                    .red_with(|it|
                    {
                        it.text(blue_potion_item.display_name());
                    })
                    .text(" for ")
                    .pink("100 Rupees")
                    .text(".")
                    ;
                })
                .end_conversation()
                .end_final_text_box();
            }));

        new.push(message(0x15EA, None, |it|
            {
                it.text("Do we have a deal?").new_line()
                .text(" ").new_line()
                .start_green_text()
                .two_choices()
                .text("Yes").new_line()
                .text("No")
                .end_final_text_box();
            }));

        // update gorman bros milk purchase
        let gorman_bros_milk_item = self.placed("ShopItemGormanBrosMilk")?;
        new.push(message(0x3463, None, |it|
            {
                it.runtime_wrap(|it|
                {
                    it.text("Won'tcha buy ")
                    .runtime_article(gorman_bros_milk_item.display(), gorman_bros_milk_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(&gorman_bros_milk_item.alternate_name(), gorman_bros_milk_item.loc());
                    })
                    .text("?")
                    ;
                })
                .disable_text_skip2()
                .end_final_text_box();
            }));
        new.push(message(0x3466, None, |it|
            {
                it.pink("50 Rupees").text(" will do ya for").runtime_pronoun_or_amount(gorman_bros_milk_item.display(), gorman_bros_milk_item.loc()).text(".").new_line()
                .text(" ").new_line()
                .start_green_text()
                .two_choices()
                .text("I'll buy ").runtime_pronoun(gorman_bros_milk_item.display(), gorman_bros_milk_item.loc()).new_line()
                .text("No thanks")
                .end_final_text_box();
            }));
        new.push(message(0x346B, None, |it|
            {
                it.runtime_wrap(|it|
                {
                    it.text("Buyin' ")
                    .runtime_article(gorman_bros_milk_item.display(), gorman_bros_milk_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(&gorman_bros_milk_item.alternate_name(), gorman_bros_milk_item.loc());
                    })
                    .text("?")
                    ;
                })
                .disable_text_skip2()
                .end_final_text_box();
            }));
        new.push(message(0x348F, None, |it|
            {
                it.runtime_wrap(|it|
                {
                    it.text("Seems like we're the only ones who have ")
                    .runtime_article(gorman_bros_milk_item.display(), gorman_bros_milk_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(&gorman_bros_milk_item.alternate_name(), gorman_bros_milk_item.loc());
                    })
                    .text(". Hyuh, hyuh. If you like, I'll sell you")
                    .runtime_pronoun_or_amount(gorman_bros_milk_item.display(), gorman_bros_milk_item.loc())
                    .text(".")
                    ;
                })
                .disable_text_skip2()
                .end_final_text_box();
            }));
        new.push(message(0x3490, None, |it|
            {
                it.pink("50 Rupees").text(" will do you for").runtime_pronoun_or_amount(gorman_bros_milk_item.display(), gorman_bros_milk_item.loc()).text("!").new_line()
                .text(" ").new_line()
                .start_green_text()
                .two_choices()
                .text("I'll buy ").runtime_pronoun(gorman_bros_milk_item.display(), gorman_bros_milk_item.loc()).new_line()
                .text("No thanks")
                .end_final_text_box();
            }));

        // update lottery message
        let lottery_item = self.placed("MundaneItemLotteryPurpleRupee")?;
        new.push(message(0x2B5C, None, |it|
            {
                it.compile_time_wrap(|wrapped|
                {
                    wrapped.text("Would you like the chance to buy your dreams for ").pink("10 Rupees").text("?");
                })
                .end_text_box()
                .runtime_wrap(|it|
                {
                    it.text("Pick any three numbers, and if those are picked, you'll win ")
                    .runtime_article(lottery_item.display(), lottery_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(lottery_item.display_name(), lottery_item.loc());
                    })
                    .text(". It's only for the ")
                    .red("first")
                    .text(" person!")
                    ;
                })
                .disable_text_skip2()
                .end_final_text_box();
            }));
        new.push(message(0x2B66, None, |it|
            {
                it.play_sound_effect(0x4853)
                .text("Congratulations!")
                .end_text_box()
                .runtime_wrap(|it|
                {
                    it.text("You win a prize of ")
                    .runtime_article(lottery_item.display(), lottery_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(lottery_item.display_name(), lottery_item.loc());
                    })
                    .text("!")
                    ;
                })
                .disable_text_skip2()
                .end_final_text_box();
            }));

        // Update Witch Blue Potion message
        let witch_blue_potion_item = self.placed("ShopItemWitchBluePotion")?;
        new.push(message(0x880, None, |it|
            {
                it.red_with(|it|
                {
                    it.runtime_item_name(witch_blue_potion_item.display_name(), witch_blue_potion_item.loc()).text(": 60 Rupees").new_line();
                })
                .text("Actually, I can't get the").new_line()
                .text("ingredients for this, so I'm sold").new_line()
                .text("out. Sorry.")
                .disable_text_box_close()
                .end_final_text_box();
            }));
        new.push(message(0x881, None, |it|
            {
                it.runtime_wrap(|it|
                {
                    it.text("What's that? You want ")
                    .runtime_article(witch_blue_potion_item.display(), witch_blue_potion_item.loc(), None)
                    .red_with(|it|
                    {
                        it.runtime_item_name(witch_blue_potion_item.display_name(), witch_blue_potion_item.loc());
                    })
                    .text(", do you?")
                    ;
                })
                .end_text_box()
                .text("Well, you gave me a mushroom, so").new_line()
                .text("I'll give you").runtime_pronoun_or_amount(witch_blue_potion_item.display(), witch_blue_potion_item.loc()).text(" for free.")
                .disable_text_skip2()
                .end_final_text_box();
            }));

        // Update Keg Challenge
        let keg_challenge_item = self.placed("ItemPowderKeg")?;
        if keg_challenge_item.item != "ItemPowderKeg"
        {
            new.push(message(0xC80, None, |it|
                {
                    it.play_sound_effect(0x38BB)
                    .runtime_wrap(|it|
                    {
                        it.text("I'm the Goron who sells ")
                        .runtime_article(keg_challenge_item.display(), keg_challenge_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(&keg_challenge_item.alternate_name(), keg_challenge_item.loc());
                        })
                        .text(", the most famous product of the Gorons.")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
            new.push(message(0xC81, None, |it|
                {
                    it.play_sound_effect(0x38FC)
                    .runtime_wrap(|it|
                    {
                        it.text("Want ")
                        .runtime_article(keg_challenge_item.display(), keg_challenge_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(keg_challenge_item.display_name(), keg_challenge_item.loc());
                        })
                        .text("? Be careful, ")
                        .runtime_article(keg_challenge_item.display(), keg_challenge_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(&keg_challenge_item.alternate_name(), keg_challenge_item.loc());
                        })
                        .text(" ")
                        .runtime_verb(keg_challenge_item.display(), keg_challenge_item.loc())
                        .text(" very dangerous...")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap_text("Until I have tested you to see if you are responsible, I can't sell to you.")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
            new.push(message(0xC83, None, |it|
                {
                    it.play_sound_effect(0x38FC)
                    .text("If you can ").red("destroy").text(" the boulder").new_line()
                    .text("that blocks the entrance to the").new_line()
                    .red("Goron Racetrack").text(" near here...")
                    .end_text_box()
                    .text("using the ").red("Powder Keg ").text("I'm about").new_line()
                    .text("to give you, then I'll sell to you.")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
            new.push(message(0xC86, None, |it|
                {
                    it.play_sound_effect(0x3A04)
                    .runtime_wrap(|it|
                    {
                        it.text("It looks like you managed to succeed! Knowing your skills, I feel fine letting you handle ")
                        .runtime_article(keg_challenge_item.display(), keg_challenge_item.loc(), None)
                        .runtime_item_name(&keg_challenge_item.alternate_name(), keg_challenge_item.loc())
                        .text(" on your own.")
                        ;
                    })
                    .end_text_box()
                    .text("It was bad of me to put you").new_line()
                    .text("through such a dangerous test. I").new_line()
                    .text("want you to take this as my").new_line()
                    .text("apology.")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
            new.push(message(0xC88, None, |it|
                {
                    it.play_sound_effect(0x38FC)
                    .text("You failed?").new_line()
                    .text("In that case, I can't sell").new_line()
                    .text("to you.")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
            new.push(message(0xC8C, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("Will you a buy ")
                        .runtime_article(keg_challenge_item.display(), keg_challenge_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(keg_challenge_item.display_name(), keg_challenge_item.loc());
                        })
                        .text(" for ").pink("100 Rupees").text("?")
                        ;
                    })
                    .new_line()
                    .start_green_text()
                    .two_choices()
                    .text("I'll buy ").runtime_pronoun(keg_challenge_item.display(), keg_challenge_item.loc()).new_line()
                    .text("No thanks")
                    .end_final_text_box();
                }));
            new.push(message(0xC8E, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("I'm the Goron who sells ")
                        .runtime_article(keg_challenge_item.display(), keg_challenge_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(&keg_challenge_item.alternate_name(), keg_challenge_item.loc());
                        })
                        .text(", the most famous product of the Gorons.")
                        ;
                    })
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("But the rules say I can't sell ")
                        .runtime_article(keg_challenge_item.display(), keg_challenge_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(&keg_challenge_item.alternate_name(), keg_challenge_item.loc());
                        })
                        .text(" to anyone who isn't a").red(" Goron").text(". ").play_sound_effect(0x391C).text("Sorry.")
                        ;
                    })
                    .end_final_text_box();
                }));
            new.push(message(0xC8F, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("The rules say I can't sell ")
                        .runtime_article(keg_challenge_item.display(), keg_challenge_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(&keg_challenge_item.alternate_name(), keg_challenge_item.loc());
                        })
                        .text(" to anyone who isn't a").red(" Goron").text(". ").play_sound_effect(0x391C).text("Sorry.")
                        ;
                    })
                    .end_final_text_box();
                }));
        }

        let big_bomb_bag_item = self.placed("UpgradeBigBombBag")?;
        new.push(message(0x29D3, None, |it|
            {
                it.play_sound_effect(0x6952)
                .text("Tonight's special bargain was just").new_line()
                .text("stolen. It just came in seconds").new_line()
                .text("ago. This is really hot!").new_line()
                .pause_text(20)
                .quick_text(|it| { it.text("I kid you not!"); })
                .end_text_box()
                .text("Actually, it's somethin' sold over").new_line()
                .text("at the ").red("Bomb Shop").text(", but a thief").new_line()
                .text("sold it to me...")
                .end_text_box()
                .runtime_wrap(|it|
                {
                    it.text("It's ")
                    .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), None)
                    .quick_text(|it|
                    {
                        it.red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text(".");
                    })
                    ;
                })
                .new_line()
                .pause_text(10)
                .text("Check it out! Buy it!")
                .disable_text_skip2()
                .end_final_text_box();
            }));

        new.push(message(0x29D7, None, |it|
            {
                it.play_sound_effect(0x6952)
                .text("Tonight's special bargain was just").new_line()
                .text("stolen. It just came in seconds").new_line()
                .text("ago. It's really hot!").new_line()
                .pause_text(20)
                .quick_text(|it| { it.text("I kid you not."); })
                .end_text_box()
                .text("Actually, it's somethin' sold over").new_line()
                .text("at the ").red("Bomb Shop").text(", but a thief").new_line()
                .text("sold it to me.")
                .end_text_box()
                .runtime_wrap(|it|
                {
                    it.text("It's ")
                    .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), None)
                    .quick_text(|it|
                    {
                        it.red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text(".");
                    })
                    ;
                })
                .new_line()
                .pause_text(10)
                .text("Check it out! Buy it!")
                .disable_text_skip2()
                .end_final_text_box();
            }));

        let all_night_mask_purchase_item = self.placed("MaskAllNight")?;
        if all_night_mask_purchase_item.item != "MaskAllNight"
        {
            new.push(message(0x29D4, None, |it|
                {
                    it.play_sound_effect(0x6952)
                    .text("Tonight's bargain is").new_line()
                    .runtime_wrap(|it|
                    {
                        it.quick_text(|it|
                        {
                            it.runtime_article(all_night_mask_purchase_item.display(), all_night_mask_purchase_item.loc(), None)
                            .red_with(|it|
                            {
                                it.runtime_item_name(all_night_mask_purchase_item.display_name(), all_night_mask_purchase_item.loc());
                            });
                        })
                        .text(" ")
                        .pause_text(10)
                        .text("for use at bedtime.")
                        ;
                    })
                    .end_text_box()
                    .text("I forgot when this was made, but").new_line()
                    .text("it sure is freaky...See?")
                    .end_text_box()
                    .text("Pretty creepy, huh?")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x29D8, None, |it|
                {
                    it.play_sound_effect(0x6952)
                    .text("Tonight's bargain is").new_line()
                    .runtime_wrap(|it|
                    {
                        it.quick_text(|it|
                        {
                            it.runtime_article(all_night_mask_purchase_item.display(), all_night_mask_purchase_item.loc(), None)
                            .red_with(|it|
                            {
                                it.runtime_item_name(all_night_mask_purchase_item.display_name(), all_night_mask_purchase_item.loc());
                            });
                        })
                        .text(" ")
                        .pause_text(10)
                        .text("for use at bedtime.")
                        ;
                    })
                    .end_text_box()
                    .text("I forgot when this was made, but").new_line()
                    .text("it sure is freaky...See?")
                    .end_text_box()
                    .text("Pretty creepy, huh?")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
        }
        Ok(())
    }
}

/// Cut a name to fit, with "..." (C# `Substring` on UTF-16 units).
pub(crate) fn shortened(name: &str, max: usize) -> String {
    let units: Vec<u16> = name.encode_utf16().collect();
    if units.len() > max {
        String::from_utf16_lossy(&units[..max - 3]) + "..."
    } else {
        name.to_string()
    }
}
