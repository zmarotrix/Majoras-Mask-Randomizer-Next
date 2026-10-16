//! Item placement for the Majora's Mask Randomizer.
//!
//! Not yet ported from MMR 2.0 (see ROADMAP.md): entrances, hints.

pub mod entrances;
pub mod fill;
pub mod gibdo;
pub mod hint_caps;
pub mod plando;
pub mod importance;
pub mod prices;
mod price_tables;
pub mod reach;
pub mod rng;
pub mod starting;
pub mod world;

pub use fill::Style;
pub use rng::Seed;

use mmr_logic::LogicFile;
use mmr_settings::Values;
use serde::Serialize;

/// How many times to retry (with a derived sub-seed) before giving up.
pub const MAX_ATTEMPTS: u32 = 50;

#[derive(Debug, Clone, Serialize)]
pub struct Placed {
    pub location: String,
    pub location_name: String,
    pub item: String,
    pub item_name: String,
    /// The item gives junk when found (e.g. it's a starting item).
    pub gives_junk: bool,
}

/// What ends up at a location, for building the ROM.
#[derive(Debug, Clone, Serialize)]
pub struct LayoutEntry {
    pub location: String,
    /// The item placed there by logic.
    pub item: String,
    /// What the player actually gets: the item, or junk in its place (for
    /// starting items, keys for open doors, removed fairies...).
    pub given: String,
    /// Placed by the randomizer (not left vanilla).
    pub randomized: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    pub seed: Seed,
    pub style: Style,
    /// Which attempt succeeded (1 = first try).
    pub attempt: u32,
    pub placements: Vec<Placed>,
    /// Important items in the order a player can collect them: each sphere
    /// needs only items from earlier spheres.
    pub spheres: Vec<Vec<Placed>>,
    /// Every location with an item, vanilla ones included.
    pub layout: Vec<LayoutEntry>,
    /// Locations that may only hold junk.
    pub junk_locations: Vec<String>,
    /// One price per `prices::message_costs()` entry (`None`: unchanged).
    pub prices: Vec<Option<u16>>,
    /// What the Ikana Castle Gibdos ask for.
    pub gibdos: Vec<gibdo::GibdoRequest>,
    /// Blitz: what the player starts with for skipped dungeons.
    pub blitz_items: Vec<String>,
    /// Picked from the random starting item groups.
    pub random_starting: Vec<String>,
    /// Moved entrances: (entrance, where it now leads from).
    pub entrances: Vec<(String, String)>,
}

/// What a seed changes before placement: the logic (wallets for prices,
/// Gibdo requests) and the choices that changed it.
pub struct Prepared {
    pub logic: LogicFile,
    pub prices: Vec<Option<u16>>,
    pub gibdos: Vec<gibdo::GibdoRequest>,
    pub blitz: starting::Blitz,
    /// Picked from the random starting item groups.
    pub random_starting: Vec<String>,
    /// Moved entrances: (entrance, where it now is).
    pub entrances: Vec<(String, String)>,
}

impl Prepared {
    /// The placement world for this seed.
    pub fn world<'a>(&self, game: &'a mmr_data::GameData, values: &Values) -> world::World<'a> {
        let mut w = world::World::new(game, &self.logic, values);
        w.apply_starting(&self.blitz, &self.random_starting);
        w
    }
}

fn is_vanilla(values: &Values) -> bool {
    values.get("LogicMode").and_then(serde_json::Value::as_str) == Some("Vanilla")
}

/// Vanilla mode: nothing is shuffled, priced or picked.
fn vanilla_values(values: &Values) -> Values {
    let mut v = values.clone();
    for (k, x) in [
        ("CustomItemList", serde_json::json!([])),
        ("PriceMode", serde_json::json!([])),
        ("RandomizeGibdoRequirements", serde_json::json!(false)),
        ("RandomStartingItemGroups", serde_json::json!([])),
        ("StartingItemMode", serde_json::json!("None")),
    ] {
        v.insert(k.into(), x);
    }
    if let Some(remains) = v.get_mut("BossRemainsMode").and_then(serde_json::Value::as_array_mut) {
        remains.retain(|f| !f.as_str().is_some_and(|s| s.starts_with("Blitz")));
    }
    v
}

/// The logic placement uses for these settings and seed.
pub fn prepare(logic: &LogicFile, values: &Values, seed: Seed) -> Result<Prepared, GenerateError> {
    let vanilla;
    let values = if is_vanilla(values) {
        vanilla = vanilla_values(values);
        &vanilla
    } else {
        values
    };
    let mut logic = mmr_logic::settings_rules::specialize(logic, values);
    let entrances = entrances::shuffle(&mut logic, values, mmr_data::game(), &mut rng::Rng::new(seed, rng::Stream::Entrances, 0)).map_err(GenerateError::Entrances)?;
    let gibdos = gibdo::randomize(&mut logic, values, &mut rng::Rng::new(seed, rng::Stream::Gibdos, 0));
    let prices = prices::randomize(&mut logic, values, &mut rng::Rng::new(seed, rng::Stream::Prices, 0));
    let mut rng = rng::Rng::new(seed, rng::Stream::StartingItems, 0);
    let lair_at = |lair: &str| entrances.iter().find(|(e, _)| e == lair).map_or_else(|| lair.to_string(), |(_, t)| t.clone());
    let blitz = starting::blitz(&mut logic, values, mmr_data::game(), &mut rng, lair_at);
    let random_starting = starting::random_groups(values, &mut rng);
    Ok(Prepared { logic, prices, gibdos, blitz, random_starting, entrances })
}

#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("couldn't make a beatable seed after {MAX_ATTEMPTS} tries. Last problem: {0}")]
    Failed(fill::FillError),
    #[error("couldn't shuffle entrances: {0}")]
    Entrances(String),
    #[error("{0}")]
    Plando(String),
}

/// Place items for `seed`. Retries with derived sub-seeds if a placement
/// paints itself into a corner.
pub fn generate(logic: &LogicFile, values: &Values, seed: Seed, style: Style) -> Result<Outcome, GenerateError> {
    generate_with(logic, values, seed, style, &[])
}

/// [`generate`], with item plando.
pub fn generate_with(logic: &LogicFile, values: &Values, seed: Seed, style: Style, plando: &[plando::PlandoCombo]) -> Result<Outcome, GenerateError> {
    let vanilla;
    let values = if is_vanilla(values) {
        vanilla = vanilla_values(values);
        &vanilla
    } else {
        values
    };
    let game = mmr_data::game();
    let prepared = prepare(logic, values, seed)?;
    let logic = &prepared.logic;
    let mut world = prepared.world(game, values);
    world.plando = plando.to_vec();
    let graph = reach::Graph::new(&world, logic, values);
    let mut last = None;
    for attempt in 0..MAX_ATTEMPTS {
        let mut rng = rng::Rng::new(seed, rng::Stream::Items, attempt);
        match fill::fill(&world, &graph, &mut rng, style, world.junk_locations.len()) {
            Ok((placement, capped)) => {
                let mut out = outcome(&world, &graph, seed, style, attempt + 1, &placement);
                // Hint caps made these junk-only.
                for l in capped {
                    let id = world.items[l].id.clone();
                    if !out.junk_locations.contains(&id) {
                        out.junk_locations.push(id);
                    }
                }
                out.prices = prepared.prices.clone();
                out.gibdos = prepared.gibdos.clone();
                out.blitz_items = blitz_items(&world, logic, &prepared, &placement);
                // Fairies added for a Great Fairy reward give junk where they are.
                let late: Vec<&String> = out.blitz_items.iter().filter(|b| !prepared.blitz.items.contains(b)).collect();
                if !late.is_empty() {
                    let junk: Vec<String> = world.junk_items.iter().filter(|i| world.pool.contains(i)).map(|&i| world.items[i].id.clone()).collect();
                    let mut rng = rng::Rng::new(seed, rng::Stream::Items, u32::MAX - 1);
                    for e in out.layout.iter_mut().filter(|e| late.contains(&&e.item)) {
                        e.given = rng.choose(&junk).cloned().unwrap_or_else(|| "RecoveryHeart".into());
                    }
                }
                out.random_starting = prepared.random_starting.clone();
                out.entrances = prepared.entrances.clone();
                return Ok(out);
            }
            Err(fill::FillError::Plando(e)) => return Err(GenerateError::Plando(e)),
            Err(e) => last = Some(e),
        }
    }
    Err(GenerateError::Failed(last.expect("at least one attempt")))
}

/// Items needed to beat the seed: removing any one of them makes the goal
/// unreachable. This is the basis of "Way of the Hero" hints.
pub fn required_items(logic: &LogicFile, values: &Values, out: &Outcome) -> Vec<String> {
    let game = mmr_data::game();
    let prepared = prepare(logic, values, out.seed).expect("the seed was generated with these settings");
    let logic = &prepared.logic;
    let world = prepared.world(game, values);
    let graph = reach::Graph::new(&world, logic, values);
    let mut placement: fill::Placement = vec![None; world.items.len()];
    for (l, item) in world.items.iter().enumerate() {
        if item.location.is_some() && world.shuffled.binary_search(&l).is_err() {
            placement[l] = Some(l);
        }
    }
    for p in &out.placements {
        placement[world.id(&p.location).unwrap()] = world.id(&p.item);
    }
    let mut required = Vec::new();
    for p in &out.placements {
        let l = world.id(&p.location).unwrap();
        let saved = placement[l].take();
        if !graph.goal_reached(&graph.sweep(&world.starting, &placement, true)) {
            required.push(p.item.clone());
        }
        placement[l] = saved;
    }
    required
}

/// A short fingerprint of a placement, for checking that a seed produces
/// exactly the same result on every platform and version.
pub fn fingerprint(out: &Outcome) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in &out.placements {
        for b in p.location.bytes().chain([b'='].into_iter()).chain(p.item.bytes()).chain([b';'].into_iter()) {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

fn outcome(world: &world::World, graph: &reach::Graph, seed: Seed, style: Style, attempt: u32, placement: &fill::Placement) -> Outcome {
    let placed = |loc: world::Id, item: world::Id| Placed {
        location: world.items[loc].id.clone(),
        location_name: world.items[loc].location.clone().unwrap_or_default(),
        item: world.items[item].id.clone(),
        item_name: world.items[item].name.clone().unwrap_or_else(|| world.items[item].id.clone()),
        gives_junk: world.gives_junk.contains(&item),
    };
    let placements = world.shuffled.iter().filter_map(|&l| placement[l].map(|i| placed(l, i))).collect();

    // Spheres: repeatedly collect everything reachable with the items found
    // so far (without chaining within a sphere), with time rules applying to
    // what's been found. Only important shuffled items are listed; vanilla
    // and junk items are collected silently.
    let mut found: fill::Placement = vec![None; placement.len()];
    let mut collected = vec![false; world.items.len()];
    let mut spheres = Vec::new();
    loop {
        let reach = graph.sweep(&world.starting, &found, true);
        let mut sphere = Vec::new();
        let mut progressed = false;
        for (l, item) in placement.iter().enumerate() {
            if let Some(i) = *item {
                if reach.locations[l] && !collected[l] {
                    collected[l] = true;
                    progressed = true;
                    found[l] = Some(i);
                    if world.shuffled.binary_search(&l).is_ok() && world.is_progression(i) && !world.gives_junk.contains(&i) {
                        sphere.push(placed(l, i));
                    }
                }
            }
        }
        if !progressed {
            break;
        }
        if !sphere.is_empty() {
            spheres.push(sphere);
        }
    }
    let layout = layout(world, seed, placement);
    let junk_locations = world.junk_locations.iter().map(|&l| world.items[l].id.clone()).collect();
    Outcome { seed, style, attempt, placements, spheres, layout, junk_locations, prices: Vec::new(), gibdos: Vec::new(), blitz_items: Vec::new(), random_starting: Vec::new(), entrances: Vec::new() }
}

/// Blitz items, plus (with remains on Great Fairy rewards) the stray
/// fairies a reward needs when a starting item ended up there: the player
/// starts with those too.
fn blitz_items(world: &world::World, logic: &LogicFile, prepared: &Prepared, placement: &fill::Placement) -> Vec<String> {
    let mut out = prepared.blitz.items.clone();
    if out.is_empty() {
        return out;
    }
    let first_fairy = world.id("CollectibleStrayFairyWoodfall1").unwrap_or(0);
    let dungeon_fairy = |f: world::Id| (first_fairy..first_fairy + 60).contains(&f);
    for reward in ["FairySpinAttack", "FairyDoubleMagic", "FairyDoubleDefense", "ItemFairySword"] {
        let Some(l) = world.id(reward) else { continue };
        let Some(item) = placement[l] else { continue };
        if !world.extra_starting.contains(&item) || !world.great_fairy_remains {
            continue;
        }
        let Some(entry) = logic.entry(reward) else { continue };
        for r in &entry.requires {
            let custom_starting = |f: world::Id| world.starting.contains(&f) && !world.extra_starting.contains(&f);
            if world.id(r).is_some_and(|f| dungeon_fairy(f) && !custom_starting(f)) && !out.contains(r) {
                out.push(r.clone());
            }
        }
    }
    out
}

/// Every location's item. Items that give junk (starting items and the
/// like) are swapped for a random junk item, except at starting locations
/// and song spots, which give a recovery heart (MMR 2.0
/// `ReplaceRecoveryHeartsWithJunk`).
fn layout(world: &world::World, seed: Seed, placement: &fill::Placement) -> Vec<LayoutEntry> {
    const STARTING_LOCATIONS: [&str; 6] = ["MaskDeku", "SongHealing", "StartingSword", "StartingShield", "StartingHeartContainer1", "StartingHeartContainer2"];
    // Junk that is itself shuffled, else any junk.
    let shuffled_junk: Vec<world::Id> = world.junk_items.iter().copied().filter(|i| world.pool.contains(i)).collect();
    let junk: Vec<world::Id> = if shuffled_junk.is_empty() { world.junk_items.iter().copied().collect() } else { shuffled_junk };
    let mut rng = rng::Rng::new(seed, rng::Stream::Items, u32::MAX);
    let mut out = Vec::new();
    for (l, item) in placement.iter().enumerate() {
        let Some(i) = *item else { continue };
        let location = &world.items[l];
        let given = if !world.gives_junk.contains(&i) {
            world.items[i].id.clone()
        } else if STARTING_LOCATIONS.contains(&location.id.as_str()) || (location.song && !world.add_songs) {
            "RecoveryHeart".to_string()
        } else {
            rng.choose(&junk).map(|&j| world.items[j].id.clone()).unwrap_or_else(|| "RecoveryHeart".to_string())
        };
        out.push(LayoutEntry { location: location.id.clone(), item: world.items[i].id.clone(), given, randomized: world.shuffled.binary_search(&l).is_ok() });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(id: &str) -> Values {
        let p = mmr_settings::presets().into_iter().find(|p| p.id == id).unwrap();
        mmr_settings::apply_preset(&p, None)
    }

    #[test]
    fn generates_beatable_seeds_for_presets() {
        let logic = mmr_logic::bundled("casual").unwrap();
        for id in ["season-1-standard", "season-2-standard", "modern", "classic"] {
            for style in [Style::Random, Style::Bespoke] {
                let out = generate(&logic, &preset(id), Seed(1), style).unwrap_or_else(|e| panic!("{id} {style:?}: {e}"));
                assert!(!out.placements.is_empty());
                assert!(!out.spheres.is_empty());
            }
        }
    }

    #[test]
    fn same_seed_same_result() {
        let logic = mmr_logic::bundled("casual").unwrap();
        let v = preset("modern");
        let a = generate(&logic, &v, Seed(99), Style::Random).unwrap();
        let b = generate(&logic, &v, Seed(99), Style::Random).unwrap();
        let c = generate(&logic, &v, Seed(100), Style::Random).unwrap();
        let key = |o: &Outcome| o.placements.iter().map(|p| (p.location.clone(), p.item.clone())).collect::<Vec<_>>();
        assert_eq!(key(&a), key(&b));
        assert_ne!(key(&a), key(&c));
    }
}
