//! From settings and a seed to a finished ROM: place items with MMR Next's
//! randomizer, then build.

use crate::layout::{LayoutItem, LogicItem, SeedLayout};
use crate::{BuildError, Plan, Stage};
use mmr_logic::LogicFile;
use mmr_randomizer::rng::{Rng, Stream};
use mmr_randomizer::{Outcome, Seed, Style};
use mmr_settings::Values;

#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    #[error(transparent)]
    Generate(#[from] mmr_randomizer::GenerateError),
    #[error(transparent)]
    Build(#[from] BuildError),
}

/// What [`build_seed`] produces.
pub struct Built {
    pub rom: Vec<u8>,
    pub outcome: Outcome,
    /// The build hash (shown as icons on the file select screen).
    pub hash: Vec<u8>,
    /// With random music, what plays where.
    pub music: Vec<crate::SongPlacement>,
    pub hash_icons: Vec<crate::hash_icons::HashIcon>,
    pub item_icons: std::collections::BTreeMap<String, Vec<u8>>,
    /// The build plan (hints, prices, traps... for the spoiler log).
    pub plan: Plan,
}

/// The build input for a randomized seed.
pub fn seed_layout(outcome: &Outcome, values: &Values, logic: &LogicFile) -> SeedLayout {
    let game = mmr_data::game();
    let prepared = mmr_randomizer::prepare(logic, values, outcome.seed).expect("the seed was generated with these settings");
    let logic = prepared.logic
        .entries
        .iter()
        .map(|e| LogicItem {
            id: e.id.clone(),
            requires: e.requires.clone(),
            any_of: e.any_of.clone(),
            fake: game.items.iter().find(|i| i.id == e.id).is_none_or(|i| i.name.is_none()),
        })
        .collect();
    let mut rng = Rng::new(outcome.seed, Stream::Cosmetics, 0);
    let mut hue = || rng.below(360) as i32;
    let (file_select_skybox, file_select_color, title_logo_color) = (hue(), hue(), hue());
    SeedLayout {
        seed: outcome.seed.0,
        settings: values.clone(),
        items: outcome.layout.iter().map(|e| LayoutItem { location: e.location.clone(), item: e.item.clone(), given: e.given.clone(), randomized: e.randomized }).collect(),
        junk_locations: outcome.junk_locations.clone(),
        logic,
        title_logo_color,
        file_select_skybox,
        file_select_color,
        message_costs: outcome.prices.iter().any(Option::is_some).then(|| outcome.prices.clone()),
        gibdos: outcome.gibdos.iter().map(|g| (g.item.clone(), g.amount, g.logic_entry.clone())).collect(),
        blitz_extra_items: outcome.blitz_items.clone(),
        random_starting_items: outcome.random_starting.clone(),
        entrances: outcome.entrances.clone(),
    }
}

/// The build plan for a randomized seed, traps included.
pub fn plan_for(outcome: &Outcome, values: &Values, logic: &LogicFile) -> Plan {
    let mut plan = Plan::from_layout(&seed_layout(outcome, values, logic));
    let prepared = mmr_randomizer::prepare(logic, values, outcome.seed).expect("the seed was generated with these settings");
    let world = prepared.world(mmr_data::game(), values);
    let is_junk = |item: &str| world.id(item).is_some_and(|i| world.is_junk(i));
    crate::traps::add_traps(&mut plan, outcome.seed.0, is_junk);
    let style = |k: &str| values.get(k).and_then(|v| v.as_str()).unwrap_or("Default").to_string();
    let vanilla = values.get("LogicMode").and_then(|v| v.as_str()) == Some("Vanilla");
    if !vanilla && (style("GossipHintStyle") != "Default" || style("GaroHintStyle") != "Default") {
        let importance = mmr_randomizer::importance::importance(logic, values, outcome, &crate::hints::stone_entries());
        let (quotes, info) = crate::hints::make_gossip_quotes(&plan, &importance, &is_junk, outcome.seed.0);
        plan.gossip_quotes = Some(quotes);
        plan.hint_info = info;
    }
    plan
}

/// Randomize and build. `rom` must be a checked, big-endian ROM (see
/// [`mmr_rom::prepare`]).
pub fn build_seed(rom: &[u8], values: &Values, logic: &LogicFile, seed: Seed, style: Style, options: &crate::Options) -> Result<Built, SeedError> {
    let outcome = mmr_randomizer::generate_with(logic, values, seed, style, &options.plando)?;
    let plan = plan_for(&outcome, values, logic);
    let out = crate::build_with(rom, &plan, Stage::Audio, options)?;
    Ok(Built { rom: out.rom, outcome, hash: out.hash, music: out.music, hash_icons: out.hash_icons, item_icons: out.item_icons, plan })
}
