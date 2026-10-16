//! Traps for MMR Next's own seeds (the old `AddTraps`): some junk becomes ice
//! traps, bomb traps, rupoors or nothing, disguised as other items.

use crate::items;
use crate::plan::{Mimic, Plan, Trap};
use mmr_randomizer::rng::{Rng, Seed, Stream};
use serde_json::Value;

const STARTING_LOCATIONS: [&str; 6] = ["MaskDeku", "SongHealing", "StartingSword", "StartingShield", "StartingHeartContainer1", "StartingHeartContainer2"];
const TRAP_TYPES: [(&str, &str); 4] = [("Ice", "IceTrap"), ("Bomb", "BombTrap"), ("Rupoor", "Rupoor"), ("Nothing", "Nothing")];

fn display_name(item: &str) -> String {
    items::name(item).unwrap_or(item).to_string()
}

/// Turn some junk into traps. `is_junk` says whether an item counts as junk
/// (the randomizer's junk items).
pub fn add_traps(plan: &mut Plan, seed: u64, is_junk: impl Fn(&str) -> bool) {
    let amount = plan.choice("TrapAmount").to_string();
    if amount.is_empty() || amount == "None" || plan.choice("LogicMode") == "Vanilla" {
        return;
    }
    let mut rng = Rng::new(Seed(seed), Stream::Traps, 0);
    let onslaught = amount == "Onslaught";
    let junk: Vec<usize> = (0..plan.items.len())
        .filter(|&i| {
            let p = &plan.items[i];
            (p.is_randomized || onslaught) && p.location.as_deref().is_some_and(|l| !STARTING_LOCATIONS.contains(&l)) && items::name(&p.item).is_some() && is_junk(&p.item)
        })
        .collect();
    let chosen = match amount.as_str() {
        "Normal" => rng.sample(&junk, junk.len() / 15),
        "Extra" => rng.sample(&junk, junk.len() / 7),
        _ => junk.clone(),
    };

    // Disguises: placed items with a model, of the kind the setting asks
    // for (randomized ones first). One per item name.
    let appearance = plan.choice("TrapAppearance").to_string();
    let highly_restricted = |item: &str| {
        (!plan.flag("AddSongs") && mmr_data::game().items.iter().any(|i| i.id == item && i.song)) || (plan.has("BossRemainsMode", "ShuffleOnly") && items::item_category(item) == Some("BossRemains"))
    };
    let mut mimics: Vec<String> = Vec::new();
    for allow_vanilla in [false, true] {
        for p in plan.items.iter().filter(|p| allow_vanilla || p.is_randomized) {
            if items::get_item_index(&p.item).is_none() || highly_restricted(&p.item) {
                continue;
            }
            let chest = items::chest_type(&p.item);
            let fits = match appearance.as_str() {
                "MajorItems" => matches!(chest, Some("LargeGold" | "SmallGold")),
                "JunkItems" => chest == Some("SmallWooden"),
                _ => true,
            };
            let name = display_name(&p.item);
            if fits && !mimics.iter().any(|m| display_name(m) == name) {
                mimics.push(p.item.clone());
            }
        }
        if !mimics.is_empty() {
            break;
        }
    }
    let weights_value = plan.setting("TrapWeights").clone();
    let weight = |t: &str| weights_value.get(t).and_then(Value::as_u64).unwrap_or(0) as u32;
    if weight("Nothing") > 0 {
        mimics.push("Nothing".into());
    }

    let mut traps = Vec::new();
    for i in chosen {
        let location = plan.items[i].location.clone().unwrap();
        let allowed: Vec<(&str, &str)> = TRAP_TYPES.iter().copied().filter(|(t, _)| *t != "Bomb" || crate::enums::items().attr(&location, "BlockBombTrapPlacement").is_none()).collect();
        let weights: Vec<u32> = allowed.iter().map(|(t, _)| weight(t)).collect();
        let pick = if weights.iter().all(|&w| w == 0) { rng.below(allowed.len()) } else { rng.weighted(&weights) };
        let (_, trap) = allowed[pick];
        let mimic_item = if trap == "Nothing" { "Nothing".to_string() } else { rng.choose(&mimics).cloned().unwrap_or_else(|| "Nothing".into()) };
        let mut mimic = Mimic { item: mimic_item.clone(), fake_name: None, chest_type: None };
        let mut name_override = None;
        if trap != "Nothing" && (items::is_model_visible(plan, &location) || items::is_text_visible(plan, &location)) {
            if trap != "Rupoor" || items::is_shop_model_visible(&location) {
                name_override = Some(format!("{} ({})", display_name(trap), display_name(&mimic_item)));
            }
            if plan.flag("TrapQuirks") && items::is_text_visible(plan, &location) {
                mimic.fake_name = Some(fake_name(&display_name(&mimic_item), &mut rng));
            }
        }
        if plan.flag("UpdateChests") {
            mimic.chest_type = Some(
                match appearance.as_str() {
                    "MajorItems" => "LargeGold",
                    "JunkItems" => "SmallWooden",
                    _ => ["LargeGold", "SmallGold", "SmallWooden"][rng.below(3)],
                }
                .to_string(),
            );
        }
        let p = &mut plan.items[i];
        p.item = trap.to_string();
        p.mimic = Some(mimic.clone());
        p.name_override = name_override;
        traps.push(Trap { item: trap.to_string(), location: Some(location), mimic: Some(mimic) });
    }
    plan.traps = Some(traps);
}

/// A plausible fake name for a shop's trap (`FakeNameUtils.CreateFakeName`).
pub fn fake_name(name: &str, rng: &mut Rng) -> String {
    let roll = rng.below(100);
    if roll <= 10 {
        if let Some((_, r)) = FULL.iter().find(|(n, _)| *n == name) {
            let options: Vec<&str> = r.split(',').collect();
            return options[rng.below(options.len())].to_string();
        }
    } else if roll <= 70 {
        let mut words: Vec<String> = name.split(' ').map(String::from).collect();
        let index = rng.below(words.len());
        if let Some((key, r)) = WORDS.iter().find(|(k, _)| words[index].starts_with(k)) {
            let options: Vec<&str> = r.split(',').collect();
            let replacement = options[rng.below(options.len())];
            words[index] = words[index].replace(key, replacement);
            return words.join(" ");
        }
    }
    replace_vowels(name, rng)
}

fn replace_vowels(name: &str, rng: &mut Rng) -> String {
    const VOWELS: [char; 5] = ['a', 'e', 'i', 'o', 'u'];
    const CENSOR: [&str; 6] = ["cum", "cunt", "dike", "penis", "puss", "shit"];
    loop {
        let mut chars: Vec<char> = name.chars().collect();
        let indexes: Vec<usize> = chars.iter().enumerate().filter(|(_, c)| VOWELS.contains(c)).map(|(i, _)| i).collect();
        for i in rng.sample(&indexes, indexes.len().min(2)) {
            let others: Vec<char> = VOWELS.iter().copied().filter(|v| *v != chars[i]).collect();
            chars[i] = others[rng.below(others.len())];
        }
        let out: String = chars.into_iter().collect();
        if !CENSOR.contains(&out.as_str()) {
            return out;
        }
    }
}

const FULL: [(&str, &str); 10] = [
    ("Captain's Hat", "Skull Mask"),
    ("Don Gero's Mask", "Frog Choir Mask"),
    ("Double Defense", "Nayru's Love"),
    ("Fire Arrows", "Din's Fire"),
    ("Goron Mask", "Goron Tunic"),
    ("Great Fairy's Mask", "Gerudo Mask"),
    ("Heart Container", "Container of Heart"),
    ("Piece of Heart", "Heart Piece"),
    ("Song of Soaring", "Farore's Wind"),
    ("Zora Mask", "Zora Tunic"),
];

const WORDS: &[(&str, &str)] = &[
    ("Attack", "Slash"),
    ("Awakening", "Ascending,Waking"),
    ("Bay", "Bae,Day"),
    ("Bean", "Plant,Seed"),
    ("Big", "Bigger,Large"),
    ("Biggest", "Best,Largest"),
    ("Blast", "Blasting,Bomb,Bomber"),
    ("Bomber", "Boomer"),
    ("Bomb", "Boom,Bomb-omb"),
    ("Boss", "Big,Master"),
    ("Bossa", "Star,Super,Tossa"),
    ("Bremen", "Brahmin,Eagle,Falco,Kass"),
    ("Bunny", "Funny,Hare,Rabbit"),
    ("Captain", "Keeta"),
    ("Circus", "Troupe"),
    ("Container", "Contaminator"),
    ("Couple's", "Divorce"),
    ("Deed", "Dead"),
    ("Deity", "Daemon,Diety,God"),
    ("Deku", "Scrub,Tree"),
    ("Don", "Lon"),
    ("Double", "Extra"),
    ("Dust", "Crust,Rust"),
    ("Elegy", "Melody"),
    ("Epona's", "Horse"),
    ("Emptiness", "Clones,Eligibility"),
    ("Empty", "Emptied,Full"),
    ("Fairy", "Pixie,Spirit"),
    ("Fierce", "Feared,Fearsome,Feisty"),
    ("Fire", "Lava,Flame"),
    ("Garo", "Gero,Ninja"),
    ("Gero", "Garo"),
    ("Giant's", "Colossus"),
    ("Gibdo", "Redead,Spooky"),
    ("Gilded", "Master"),
    ("Goht", "Goat"),
    ("Gold", "Golden,Shiny"),
    ("Goron", "Darmani,Darunia,Ganon"),
    ("Great", "Grate"),
    ("Gyorg", "Fish,Greg"),
    ("Hat", "Cap,Hood,Mask"),
    ("Healing", "Feeling,Sealing"),
    ("Heart", "Hate,Life"),
    ("Hero's", "Fairy,Hylian,Zero's"),
    ("Hood", "Hat,Mask"),
    ("Hookshot", "Longshot"),
    ("Ice", "Freeze,Icy"),
    ("Kafei", "Cafe,Kafie"),
    ("Kamaro", "Dancer,Karamo"),
    ("Keaton", "Keeton,Pikachu"),
    ("Keg", "Bomb,Explosive"),
    ("Large", "Big,Larger"),
    ("Largest", "Best,Biggest"),
    ("Letter", "Message,Parcel"),
    ("Light", "Lightning,Lemon"),
    ("Magic", "Magical"),
    ("Mama", "Mom,Mother"),
    ("Mask", "Hat,Hood"),
    ("Memories", "Dreams,Memoires"),
    ("Night", "Day"),
    ("Ocean", "Water,River"),
    ("Odolwa", "Odalwa"),
    ("Order", "Giants"),
    ("Pendant", "Jewel,Locket"),
    ("Pictobox", "Camerabox,Picto Box,Picturebox"),
    ("Piece", "Part,Peace"),
    ("Postman's", "Delivery,Mailman's"),
    ("Potion", "Elixir,Medicine,Tonic"),
    ("Powder", "Power,Super"),
    ("Power", "Energy"),
    ("Purple", "Violet"),
    ("Remains", "Fragments,Remnants"),
    ("Romani", "Roman,Moroni"),
    ("Room", "Door,Inn"),
    ("Scents", "Smells"),
    ("Shield", "Defense"),
    ("Silver", "Large"),
    ("Skulltula", "Spider"),
    ("Small", "Dungeon"),
    ("Snowhead", "Snowdead"),
    ("Soaring", "Flying,Warping"),
    ("Sonata", "Ballad,Santa"),
    ("Song", "Sound"),
    ("Spin", "Spain,Spinning"),
    ("Spirit", "Essence,Soul"),
    ("Stick", "Branch"),
    ("Stone", "Rock,Stoned"),
    ("Storms", "Rain,Thunder"),
    ("Stray", "Lost"),
    ("Swamp", "Forest,Woods"),
    ("Sword", "Blade"),
    ("Tear", "Jewel,Stone"),
    ("Temple", "Dungeon"),
    ("Title", "Tidal"),
    ("Town", "City,Village"),
    ("Truth", "Lying,Sight"),
    ("Twinmold", "Twinfold,Twinworm"),
    ("Wave", "Cave,Wade"),
    ("Woodfall", "Woodfail,Woodtall"),
    ("Zora", "Mikau,Ruto"),
    ("to", "for"),
];
