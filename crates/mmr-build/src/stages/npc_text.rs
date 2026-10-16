//! NPC text that hints at where items are (`WriteNPCHintText`).

use crate::message::{message, MessageEntry};
use crate::{BuildError, Builder};

impl Builder<'_> {
    pub(crate) fn write_npc_hint_text(&mut self, new: &mut Vec<MessageEntry>) -> Result<(), BuildError> {
        /*

        The mask salesman said that if
        you got back the precious thing
        that was stolen from you, he
        could return you to normal!
        Did you completely forget or
        what?

        ---

        Go to the shrine near the
        North Gate. You'll find the
        Great Fairy in there!

        ---

        Hey, aren't you going to the
        Great Fairy's shrine near the
        North Gate?

        ---

        Quick! We have to find the
        Stray Fairy in town and return
        the Great Fairy to normal!

        ---

        What are you doing?
        Aren't you gonna take this fairy
        to the shrine near the North
        Gate?

        ---

        He said the secret route is in
        East Clock Town...
        So why aren't you going there?

        */

        let bank1_item = self.placed("UpgradeAdultWallet")?;
        if bank1_item.is_randomized
        {
            new.push(message(0x44D, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("For example, if you deposit ").pink("200 Rupees").text(", you'll get ")
                        .runtime_article(bank1_item.display(), bank1_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(bank1_item.display_name(), bank1_item.loc());
                        })
                        .text(".")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x47A, None, |it|
                {
                    it.compile_time_wrap_text("See! Wasn't that a great incentive for saving money? Keep at it and you'll get another special gift in no time!")
                    .end_final_text_box();
                }));
        }

        let bank2_item = self.placed("MundaneItemBankBlueRupee")?;
        if bank2_item.item != "MundaneItemBankBlueRupee"
        {
            new.push(message(0x47B, None, |it|
                {
                    it.compile_time_wrap_text("That's what they call a rewards program!")
                    .end_final_text_box();
                }));
        }

        let romani_game_item = self.placed("SongEpona")?;
        if romani_game_item.is_randomized
        {
            new.push(message(0x4C1, None, |it|
                {
                    it.three_choices()
                    .start_green_text()
                    .text(romani_game_item.display_name()).new_line()
                    .text("Song of Feelings").new_line() // purposely changed from "Song of Healing" to avoid conflicting answers
                    .text("Song of the Field")
                    .end_final_text_box();
                }));
        }

        let milk_bar_chateau_item = self.placed("ShopItemMilkBarChateau")?;
        if milk_bar_chateau_item.item != "ShopItemMilkBarChateau"
        {
            new.push(message(0x4D7, None, |it|
                {
                    it.three_choices()
                    .start_green_text()
                    .text("Romani Run").new_line()
                    .text(milk_bar_chateau_item.display_name()).new_line()
                    .text("Chateau Moroni")
                    .end_final_text_box();
                }));

            new.push(message(0x2AFA, None, |it|
                {
                    it.text("We're called a ").red("Milk Bar").text(", and we").new_line()
                    .text("serve the ").dark_blue("item of the night").text(".")
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("Our most popular, of course, ")
                        .runtime_verb(milk_bar_chateau_item.display(), milk_bar_chateau_item.loc())
                        .text(" ")
                        .runtime_article(milk_bar_chateau_item.display(), milk_bar_chateau_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(milk_bar_chateau_item.display_name(), milk_bar_chateau_item.loc());
                        })
                        .text(". Vintage.")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap(|it|
                    {
                        it.text("We source ")
                        .runtime_pronoun(milk_bar_chateau_item.display(), milk_bar_chateau_item.loc())
                        .text(" from Romani Ranch, and the current price is ")
                        .pink("200 Rupees")
                        .text("!")
                        ;
                    })
                    .end_text_box()
                    .text("...You're not surprised?")
                    .end_conversation()
                    .end_final_text_box();
                }));
        }

        /*

        Please find a way to return me to
        the Fairy Fountain in North Clock
        Town.

        */

        let big_bomb_bag_item = self.placed("UpgradeBigBombBag")?;
        if big_bomb_bag_item.is_randomized
        {
            new.push(message(0x648, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("We're expecting to get ")
                        .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), Some("our "))
                        .red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text(" back in stock pretty soon...")
                        ;
                    })
                    .end_text_box()
                    .text("But now that I think about it, it's").new_line()
                    .text("already late...")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x649, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("We just got ")
                        .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text(" in stock.")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x64A, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("Actually, we should've had ")
                        .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text(" in stock, but it seems there was an accident getting it here to the store.")
                        ;
                    })
                    .end_text_box()
                    .text("I don't know when we'll be getting").new_line()
                    .text("it now...")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x65A, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("A nice fella helped me out, so we can finally sell ")
                        .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text("!")
                        ;
                    })
                    .end_text_box()
                    .text("Once again, the Giants are looking").new_line()
                    .text("out for me!")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x65B, None, |it|
                {
                    it.text("Umm, Mommy...").new_line()
                    .runtime_wrap(|it|
                    {
                        it.text("Don't go picking up ")
                        .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text(" in the middle of the night anymore.")
                        ;
                    })
                    .end_text_box()
                    .text("It's like asking to be mugged.").new_line()
                    .end_text_box()
                    .text("And I've actually heard that a").new_line()
                    .text("thief has been lurking on the").new_line()
                    .text("outskirts of town...")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x65C, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("But it's been our lifelong dream to sell ")
                        .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text(" since back in your Daddy's day.")
                        ;
                    })
                    .end_text_box()
                    .text("Since we can't get the Goron-made").new_line()
                    .text("goods, this was our big chance,").new_line()
                    .text("sonny!")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x65E, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("Please try ")
                        .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), Some("our "))
                        .red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text(".")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x660, None, |it|
                {
                    it.text("It's such a shame... ").new_line()
                    .runtime_wrap(|it|
                    {
                        it.text("I thought we could finally sell ")
                        .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text("...")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x661, None, |it|
                {
                    it.text("Umm, Mommy...").new_line()
                    .runtime_wrap(|it|
                    {
                        it.text("Don't go picking up ")
                        .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text(" in the middle of the night anymore.")
                        ;
                    })
                    .end_text_box()
                    .text("It's like asking to be mugged.").new_line()
                    .end_text_box()
                    .text("And doesn't the North Gate have").new_line()
                    .text("a reputation for being a dangerous").new_line()
                    .text("place?")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x2A32, None, |it|
                {
                    it.play_sound_effect(0x6909)
                    .runtime_wrap(|it|
                    {
                        it.text("Oh my. And I thought we would finally be able to stock ")
                        .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text(" in our store. What a shame...")
                        ;
                    })
                    .end_conversation()
                    .end_final_text_box();
                }));

            new.push(message(0x2A33, None, |it|
                {
                    it.play_sound_effect(0x6909)
                    .text("Oh my. You think it'll never").new_line()
                    .text("happen to you. Well, now I've").new_line()
                    .text("learned my lesson.")
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("And I thought we would finally be able to stock ")
                        .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text(" in our shop. It's too bad...")
                        ;
                    })
                    .end_conversation()
                    .end_final_text_box();
                }));

            new.push(message(0x2A34, None, |it|
                {
                    it.play_sound_effect(0x690A)
                    .runtime_wrap(|it|
                    {
                        it.text("Thank you. Since he didn't make off with them, I can finally stock ")
                        .runtime_article(big_bomb_bag_item.display(), big_bomb_bag_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(big_bomb_bag_item.display_name(), big_bomb_bag_item.loc());
                        })
                        .text(" at our shop. Maybe I'll put 'em out tomorrow.")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
        }

        let keg_item = self.obj("ItemPowderKeg")?;
        if keg_item.is_randomized && keg_item.item == "ItemPowderKeg"
        {
            let (region_preposition, region_name) = self.region_hint(keg_item.loc());

            new.push(message(0x67D, None, |it|
                {
                    it.play_sound_effect(0x3ABB)
                    .runtime_wrap(|it|
                    {
                        it.text("OK, listen here. You didn't hear it from me, but there's a ")
                        .red("Powder Keg")
                        .text(" somewhere ").text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref()).text(".")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap_text("Come back here after you learn to use that, then I can sell you one.")
                    .end_final_text_box();
                }));

            new.push(message(0x680, None, |it|
                {
                    it.play_sound_effect(0x3ABB)
                    .runtime_wrap(|it|
                    {
                        it.text("OK, listen here. You didn't hear it from me, but there's a ")
                        .red("Powder Keg")
                        .text(" somewhere ").text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref()).text(".")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap_text("Come back here after you learn to use that, then I can sell you one.")
                    .end_text_box()
                    .compile_time_wrap_text("But the next time you come, I might not be here.")
                    .end_final_text_box();
                }));

            new.push(message(0x681, None, |it|
                {
                    it.play_sound_effect(0x3ABB)
                    .runtime_wrap(|it|
                    {
                        it.text("Quick! Find the ")
                        .red("Powder Keg")
                        .text(" somewhere ").text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref())
                        .text(".")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap_text("Learn how to use it.")
                    .disable_text_skip()
                    .end_final_text_box();
                }));

            new.push(message(0x683, None, |it|
                {
                    it.play_sound_effect(0x3ABB)
                    .runtime_wrap(|it|
                    {
                        it.text("Quick! Find the ")
                        .red("Powder Keg")
                        .text(" somewhere ").text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref())
                        .text(".")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap_text("Learn how to use it.")
                    .disable_text_skip()
                    .end_final_text_box();
                }));
        }

        /*

        This is the Bombers' Notebook.
        It contains the words we live by!
        Read it over!

        1. Find troubled people and add
            their names and pictures.
            Only 20 people will fit in
            your book.
        2. Promise to help them.
            Mark promises with Promise
            Stickers. Never be late with
            fulfilling your promises.
        3. Whenever you solve someone's
            problem, it makes you happy,
            so a Happy Sticker will be
            added to your book.
        4. No removing stickers!
            Use Promise Stickers to keep
            track of people until everyone
            is happy.
        Don't forget the rules!

        */

        let good_picto_contest_item = self.placed("MundaneItemPictographContestRedRupee")?;
        if good_picto_contest_item.is_randomized
        {
            new.push(message(0x9CD, None, |it|
                {
                    it.text("Now then, please choose your").new_line()
                    .text("prize:").new_line()
                    .two_choices()
                    .start_green_text()
                    .runtime_item_name(good_picto_contest_item.display_name(), good_picto_contest_item.loc()).new_line()
                    .text("Another boat cruise")
                    .end_final_text_box();
                }));
        }

        let standard_picto_contest_item = self.placed("MundaneItemPictographContestBlueRupee")?;
        if standard_picto_contest_item.is_randomized
        {
            new.push(message(0x9D5, None, |it|
                {
                    it.text("All right...Choose your prize").new_line()
                    .text(" ").new_line()
                    .two_choices()
                    .start_green_text()
                    .runtime_item_name(standard_picto_contest_item.display_name(), standard_picto_contest_item.loc()).new_line()
                    .text("Another boat cruise")
                    .end_final_text_box();
                }));
        }

        /*

        Huh? Haven't you gotten a
        pictograph box yet?

        If you take the boat cruise, they'll
        give you one for sure. Go ask at
        that window.

        */

        let gold_dust_item = self.obj("ItemBottleGoronRace")?;
        if gold_dust_item.is_randomized && gold_dust_item.item == "ItemBottleGoronRace"
        {
            let (region_preposition, region_name) = self.region_hint(gold_dust_item.loc());

            new.push(message(0xC49, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("What? What?!? You say that ").red("gold dust").text(" can be found ")
                        .text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref())
                        .text("? How do you even know that?")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0xC4A, None, |it|
                {
                    it.text("What if you tried searching there?")
                    .end_conversation()
                    .end_final_text_box();
                }));
        }

        /*

        But...
        If you have gold dust, I'll buy it
        off you for 40 Rupees.
        How about it?

        ---

        So, come on...Will you sell it for
        40 Rupees?

        ---

        See, it was long ago when I used
        to use this thing called a
        hookshot to catch fish off the
        coast...

        But I was attacked by these
        pirates, and they took it away
        with them.

        They say...
        it's a legendary treasure..

        ---

        If you're set on meeting those
        beautiful pirates, try going to the
        valley on the other side of town!

        I've heard there's a mask there
        that lets the wearer escape
        detection.

        */

        let beaver_race1_item = self.placed("ItemBottleBeavers")?;
        if beaver_race1_item.is_randomized
        {
            new.push(message(0x10CF, None, |it|
                {
                    it.play_sound_effect(0x291A)
                    .runtime_wrap(|it|
                    {
                        it.text("You want ")
                        .runtime_article(beaver_race1_item.display(), beaver_race1_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(beaver_race1_item.display_name(), beaver_race1_item.loc());
                        })
                        .text(", don't you? You never learn your lesson!");
                    })
                    .disable_text_skip()
                    .end_final_text_box();
                }));

            new.push(message(0x10D0, None, |it|
                {
                    it.play_sound_effect(0x291A)
                    .text("Koo, koo, koo.").new_line()
                    .text("OK, I'll give you")
                    .runtime_pronoun_or_amount(beaver_race1_item.display(), beaver_race1_item.loc())
                    .text(".")
                    .end_text_box()
                    .text("But...")
                    .disable_text_skip()
                    .end_final_text_box();
                }));

            new.push(message(0x10D4, None, |it|
                {
                    it.play_sound_effect(0x291A)
                    .text("Koo, koo, koo.").new_line()
                    .runtime_wrap(|it|
                    {
                        it.text("If you want ")
                        .runtime_article(beaver_race1_item.display(), beaver_race1_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(beaver_race1_item.display_name(), beaver_race1_item.loc());
                        })
                        .text(", I can give you one...");
                    })
                    .end_text_box()
                    .compile_time_wrap(|it|
                    {
                        it.text("But only if you can swim through all the ")
                        .red("rings")
                        .text(" in the river in under ")
                        .red("two minutes")
                        .text(".")
                        ;
                    })
                    .disable_text_skip()
                    .end_final_text_box();
                }));

            new.push(message(0x10E0, None, |it|
                {
                    it.play_sound_effect(0x2919)
                    .text("What?").new_line()
                    .pause_text(20)
                    .runtime_wrap(|it|
                    {
                        it.text("He wants ")
                        .runtime_article(beaver_race1_item.display(), beaver_race1_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(beaver_race1_item.display_name(), beaver_race1_item.loc());
                        })
                        .text("?");
                    })
                    .disable_text_skip()
                    .end_final_text_box();
                }));

            new.push(message(0x10E1, None, |it|
                {
                    it.play_sound_effect(0x291A)
                    .runtime_wrap(|it|
                    {
                        it.text("Yeah... ")
                        .runtime_article(beaver_race1_item.display(), beaver_race1_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(beaver_race1_item.display_name(), beaver_race1_item.loc());
                        })
                        .text(".")
                        ;
                    })
                    .disable_text_skip()
                    .end_final_text_box();
                }));

            new.push(message(0x10E3, None, |it|
                {
                    it.play_sound_effect(0x2919)
                    .runtime_wrap(|it|
                    {
                        it.text("To us, ")
                        .runtime_article(beaver_race1_item.display(), beaver_race1_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(beaver_race1_item.display_name(), beaver_race1_item.loc());
                        })
                        .text(" ")
                        .runtime_verb(beaver_race1_item.display(), beaver_race1_item.loc())
                        .text(" a treasure.")
                        ;
                    })
                    .end_text_box()
                    .text("I can't give you")
                    .runtime_pronoun_or_amount(beaver_race1_item.display(), beaver_race1_item.loc())
                    .text(" just because").new_line()
                    .text("you beat my little brother.")
                    .disable_text_skip()
                    .end_final_text_box();
                }));

            new.push(message(0x10E4, None, |it|
                {
                    it.play_sound_effect(0x291A)
                    .text("We can't just give you")
                    .runtime_pronoun_or_amount(beaver_race1_item.display(), beaver_race1_item.loc())
                    .text(".")
                    .disable_text_skip()
                    .end_final_text_box();
                }));

            new.push(message(0x10E5, None, |it|
                {
                    it.play_sound_effect(0x2919)
                    .text("I'll give you")
                    .runtime_pronoun_or_amount(beaver_race1_item.display(), beaver_race1_item.loc())
                    .text(" after you race").new_line()
                    .text("once more against me.")
                    .end_text_box()
                    .text("So, will you try?").new_line()
                    .text(" ").new_line()
                    .start_green_text()
                    .two_choices()
                    .text("Sure").new_line()
                    .text("No thanks")
                    .end_final_text_box();
                }));

            new.push(message(0x125F, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("Hey, aren't you Mikau? Were you able to get ")
                        .runtime_article(beaver_race1_item.display(), beaver_race1_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(beaver_race1_item.display_name(), beaver_race1_item.loc());
                        })
                        .text(" from the beavers on top of the ").red("waterfall").text("?");
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x1261, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("But what do you need ")
                        .runtime_article(beaver_race1_item.display(), beaver_race1_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(beaver_race1_item.display_name(), beaver_race1_item.loc());
                        })
                        .text(" for, Mikau?");
                    })
                    .end_conversation()
                    .end_final_text_box();
                }));
        }

        let beaver_race2_item = self.placed("HeartPieceBeaverRace")?;
        if beaver_race2_item.is_randomized
        {
            new.push(message(0x10F5, None, |it|
                {
                    it.play_sound_effect(0x2919)
                    .runtime_wrap(|it|
                    {
                        it.text("But all we have left ")
                        .runtime_verb(beaver_race2_item.display(), beaver_race2_item.loc())
                        .text(" ")
                        .runtime_article(beaver_race2_item.display(), beaver_race2_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(beaver_race2_item.display_name(), beaver_race2_item.loc());
                        })
                        .text("...right, little brother?")
                        ;
                    })
                    .disable_text_skip()
                    .end_final_text_box();
                }));

            new.push(message(0x10F6, None, |it|
                {
                    it.play_sound_effect(0x291A)
                    .compile_time_wrap_text("Yes, that's all we have, big brother.")
                    .disable_text_skip()
                    .end_final_text_box();
                }));
        }

        let ocean_spider_house_day1_item = self.placed("UpgradeGiantWallet")?;
        let ocean_spider_house_day2_item = self.placed("MundaneItemOceanSpiderHouseDay2PurpleRupee")?;
        let ocean_spider_house_day3_item = self.placed("MundaneItemOceanSpiderHouseDay3RedRupee")?;
        if ocean_spider_house_day1_item.is_randomized || ocean_spider_house_day2_item.is_randomized || ocean_spider_house_day3_item.is_randomized
        {
            new.push(message(0x1131, None, |it|
                {
                    it.text("It's gotten bigger again.").new_line()
                    .text("Th-this is bad...")
                    .end_text_box()
                    .compile_time_wrap_text("I'll have to sell my life savings to find somewhere to hide!")
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("But all I have left is ")
                        .runtime_article(ocean_spider_house_day1_item.display(), ocean_spider_house_day1_item.loc(), Some("my "))
                        .red_with(|it|
                        {
                            it.runtime_item_name(ocean_spider_house_day1_item.display_name(), ocean_spider_house_day1_item.loc());
                        })
                        .text("...")
                        ;
                    })
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("Though maybe someone will be happy with ")
                        .runtime_article(ocean_spider_house_day2_item.display(), ocean_spider_house_day2_item.loc(), Some("my "))
                        .red_with(|it|
                        {
                            it.runtime_item_name(ocean_spider_house_day2_item.display_name(), ocean_spider_house_day2_item.loc());
                        })
                        .text("...")
                        ;
                    })
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("Or I can try ")
                        .runtime_article(ocean_spider_house_day3_item.display(), ocean_spider_house_day3_item.loc(), Some("my "))
                        .red_with(|it|
                        {
                            it.runtime_item_name(ocean_spider_house_day3_item.display_name(), ocean_spider_house_day3_item.loc());
                        })
                        .text(" as a last resort...")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap_text("Well... I'd better start searching.")
                    .end_conversation()
                    .end_final_text_box();
                }));
        }
        if ocean_spider_house_day1_item.is_randomized
        {
            new.push(message(0x113B, None, |it|
                {
                    it.text("If only you could have done").new_line()
                    .text("something about this place").new_line()
                    .text("yesterday...")
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("I could have given you ")
                        .runtime_article(ocean_spider_house_day1_item.display(), ocean_spider_house_day1_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(ocean_spider_house_day1_item.display_name(), ocean_spider_house_day1_item.loc());
                        })
                        .text("...")
                        ;
                    })
                    .end_final_text_box();
                }));

            new.push(message(0x1140, None, |it|
                {
                    it.text("But dang, if only you could have").new_line()
                    .text("done something ").red("two days ago").text("...")
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("I could have given you ")
                        .runtime_article(ocean_spider_house_day1_item.display(), ocean_spider_house_day1_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(ocean_spider_house_day1_item.display_name(), ocean_spider_house_day1_item.loc());
                        })
                        .text("...")
                        ;
                    })
                    .end_final_text_box();
                }));
        }

        let beaver_bottle_item = self.obj("ItemBottleBeavers")?;
        if beaver_bottle_item.is_randomized
        {
            let (region_preposition, region_name) = self.region_hint(beaver_bottle_item.loc());

            new.push(message(0x1240, None, |it|
                {
                    it.compile_time_wrap(|it|
                    {
                        it.text("I told you that there's an ").red("Empty Bottle").text(" somewhere ")
                        .text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref())
                        .text(", right?")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap_text("If you don't have any yet, Mikau, you should go there before heaging off to the Pirates' Fortress.")
                    .end_conversation()
                    .end_final_text_box();
                }));

            new.push(message(0x1242, None, |it|
                {
                    it.compile_time_wrap(|it|
                    {
                        it.text("I already told you that there's an ").red("Empty Bottle").text(" somewhere ")
                        .text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref())
                        .text(", right?")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap_text("If you don't have any yet, you should go there before heaging off to the Pirates' Fortress.")
                    .end_conversation()
                    .end_final_text_box();
                }));
        }

        let storms_tablet_item = self.placed("SongStorms")?;
        if storms_tablet_item.is_randomized
        {
            new.push(message(0x13F9, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("Here lies ")
                        .runtime_article(storms_tablet_item.display(), storms_tablet_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(storms_tablet_item.display_name(), storms_tablet_item.loc());
                        })
                        .text(".")
                        ;
                    })
                    .end_final_text_box();
                }));
        }

        let dampe_digging_item = self.placed("ItemBottleDampe")?;
        if dampe_digging_item.is_randomized
        {
            new.push(message(0x13FA, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("Here lies ")
                        .runtime_article(dampe_digging_item.display(), dampe_digging_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(dampe_digging_item.display_name(), dampe_digging_item.loc());
                        })
                        .text(".")
                        ;
                    })
                    .end_final_text_box();
                }));
        }

        let graveyard_night2_item = self.placed("HeartPieceKnuckle")?;
        if graveyard_night2_item.is_randomized
        {
            new.push(message(0x13FB, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("Here lies ")
                        .runtime_article(graveyard_night2_item.display(), graveyard_night2_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(graveyard_night2_item.display_name(), graveyard_night2_item.loc());
                        })
                        .text(".")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap_text("When one who possesses eyes that can see the truth arrives, the treasure far below shall awaken.")
                    .end_final_text_box();
                }));
        }

        /*

        Look!
        There's a mask there!!!

        It's the Sun's Mask!

        ---

        I got the Sun's Mask back!

        ---

        It's 'cause I'm about as impressive
        as a stone, right?...I'm used to it,
        though.

        ---

        I am no longer part of the
        living...My sadness to the moon...
        I haven't left my dance to the
        world...I am filled with regret.
        (Translation)             
        I am disappointed, oh moon.
        I have died!

        Oh, I planned to bring the world
        together and stir it into a giant
        melting pot with my dance!

        If only I had taught my new dance
        to someone...

        */

        let garo_mask_item = self.obj("MaskGaro")?;
        if garo_mask_item.is_randomized
        {
            let (region_preposition, region_name) = self.region_hint(garo_mask_item.loc());

            new.push(message(0x14EE, None, |it|
                {
                    it.text("Yee-hee-hee.").new_line()
                    .text("Do you have your own ").red("horse").text("?")
                    .end_text_box()
                    .compile_time_wrap(|it|
                    {
                        it.text("And if you are not wearing the ")
                        .red("mask")
                        .text(" that houses the ")
                        .red("wandering spirits...")
                        ;
                    })
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("Which can be found ")
                        .text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref())
                        .text("...")
                        ;
                    })
                    .end_text_box()
                    .text("I will not let you pass.").new_line()
                    .text("Yee-hee-hee.")
                    .end_conversation()
                    .end_final_text_box();
                }));

            new.push(message(0x14F2, None, |it|
                {
                    it.compile_time_wrap(|it|
                    {
                        it.text("But if you must enter, then you must obtain the ")
                        .red("mask")
                        .text(" containing ")
                        .red("wandering spirits...")
                        ;
                    })
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("That can be found ")
                        .text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref())
                        .text("...")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x14F4, None, |it|
                {
                    it.text("Yee-hee-hee.").new_line()
                    .text("Do you have your own ").red("horse").text("?")
                    .end_text_box()
                    .compile_time_wrap(|it|
                    {
                        it.text("And if you are not wearing the ")
                        .red("mask")
                        .text(" containing ")
                        .red("wandering spirits...")
                        ;
                    })
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("Which can be found ")
                        .text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref())
                        .text("...")
                        ;
                    })
                    .end_text_box()
                    .text("I will not let you pass.").new_line()
                    .text("Yee-hee-hee.")
                    .end_conversation()
                    .end_final_text_box();
                }));
        }

        let song_of_soaring_item = self.obj("SongSoaring")?;
        if song_of_soaring_item.is_randomized && song_of_soaring_item.item == "SongSoaring"
        {
            let (region_preposition, region_name) = self.region_hint(song_of_soaring_item.loc());

            new.push(message(0xBFC, None, |it|
                {
                    it.compile_time_wrap(|it|
                    {
                        it.text("If you have left ").red("proof ").text("of our encounter on any of those ").red("stone statues...");
                    })
                    .end_text_box()
                    .compile_time_wrap(|it|
                    {
                        it.text("Then the ").red("song").text(" ")
                        .text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref())
                        .text(" will certainly be of some assistance...");
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            self.apply_mod("mods", "misc_changes_restore_swamp_owl")?;
        }

        let song_of_time_item = self.obj("SongTime")?;
        if song_of_time_item.is_randomized && song_of_time_item.item == "SongTime"
        {
            let (region_preposition, region_name) = self.region_hint(song_of_time_item.loc());

            new.push(message(0x164A, None, |it|
                {
                    it.quick_text(|it| { it.text("Oh, yeah!"); }).new_line()
                    .text("Now listen up!")
                    .end_text_box()
                    .compile_time_wrap(|it|
                    {
                        it.text("There's a mysterious song ")
                        .text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref())
                        .text(", and if you ").red("play").text(" that song ").red("backwards")
                        .text(", you can ").red("slow").text(" the flow of time.")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap(|it|
                    {
                        it.text("And if you play each note ").red("twice").text(" in a row, you can move a ")
                        .red("half day").text(" forward in time!")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x1658, None, |it|
                {
                    it.quick_text(|it| { it.text("Oh, yeah!"); }).new_line()
                    .text("Now listen up!")
                    .end_text_box()
                    .compile_time_wrap(|it|
                    {
                        it.text("There's a strange song ")
                        .text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref())
                        .text(", and if you ").red("play").text(" that song ").red("backwards")
                        .text(", you can ").red("slow").text(" the flow of time.")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap(|it|
                    {
                        it.text("And if you play each note ").red("twice").text(" in a row, you can move a ")
                        .red("half day").text(" forward through time!")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));

            new.push(message(0x165E, None, |it|
                {
                    it.quick_text(|it| { it.text("Oh, yeah!"); }).new_line()
                    .text("Now listen up!")
                    .end_text_box()
                    .compile_time_wrap(|it|
                    {
                        it.text("There's a mysterious song ")
                        .text(region_preposition.as_deref().unwrap_or("")).red_opt(region_name.as_deref())
                        .text(", and it seems if you ").red("play").text(" that song ").red("backwards")
                        .text(", you can ").red("slow").text(" the passage of time.")
                        ;
                    })
                    .end_text_box()
                    .compile_time_wrap(|it|
                    {
                        it.text("And if you play each note ").red("twice").text(" in a row, you can move a ")
                        .red("half day").text(" forward through time!")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
        }

        /*

        Patron Guidelines for the 
        Milk Bar, Latte:

        We offer our customers limited-
        run milk. Thus, we are a members-
        only establishment.

        Those who do not have proof of
        membership will be refused
        service.
                     The Owner

        ---

        If you can get back the precious
        item that was stolen from you,
        I will return you to normal.

        ---

        Oh! Oh! Ohhh!!!
        You got it! You got it!
        You got it! You got it!!!

        Then listen to me. Please play this
        song that I am about to perform,
        and remember it well...

        This is a melody that heals evil
        magic and troubled spirits, turning
        them into masks.

        I am sure it will be of assistance
        to you in the future.

        Ah, yes. I give you this mask in
        commemoration of this day.

        Fear not, for the magic has been
        sealed inside the mask.

        When you wear it, you will
        transform into the shape you just
        were. When you remove it, you
        will return to normal.

        ---

        Green hat...
        Green clothes...

        */

        let curiosity_shop_man1_item = self.placed("MaskKeaton")?;
        if curiosity_shop_man1_item.item != "MaskKeaton"
        {
            new.push(message(0x29E2, None, |it|
                {
                    it.text("Now Kafei...").new_line()
                    .text("I've known him since he was real").new_line()
                    .text("little...")
                    .end_text_box()
                    .text("But when he showed up looking all").new_line()
                    .text("young in that little ").red("brat body").text(",").new_line()
                    .text("I didn't know what I was seeing!").new_line()
                    .text(" ")
                    .end_text_box()
                    .runtime_wrap(|it|
                    {
                        it.text("All it took was one glance at the ")
                        .red_with(|it|
                        {
                            it.runtime_item_name(curiosity_shop_man1_item.display_name(), curiosity_shop_man1_item.loc());
                        })
                        .text(" he was carrying for me to realize that I was looking at my old friend.")
                        ;
                    })
                    .end_text_box()
                    .text("I gave ").runtime_pronoun(curiosity_shop_man1_item.display(), curiosity_shop_man1_item.loc()).text(" to him a long time").new_line()
                    .text("ago when he was just li'l Kafei").new_line()
                    .text("Didn't know he kept ").runtime_pronoun(curiosity_shop_man1_item.display(), curiosity_shop_man1_item.loc()).text(" that well").new_line()
                    .text("for so long...")
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
        }

        // Update Keg Challenge
        let keg_challenge_item = self.placed("ItemPowderKeg")?;
        if keg_challenge_item.item != "ItemPowderKeg"
        {
            new.push(message(0xDEE, None, |it|
                {
                    it.runtime_wrap(|it|
                    {
                        it.text("There's a Goron in this village who sells ")
                        .runtime_article(keg_challenge_item.display(), keg_challenge_item.loc(), None)
                        .red_with(|it|
                        {
                            it.runtime_item_name(&keg_challenge_item.alternate_name(), keg_challenge_item.loc());
                        })
                        .text(".")
                        ;
                    })
                    .disable_text_skip2()
                    .end_final_text_box();
                }));
        }
        Ok(())
    }
}
