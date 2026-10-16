//! Which placed items matter, for hints and the spoiler log.
//!
//! - *Required*: without it the seed can't be beaten ("Way of the Hero").
//! - *Important*: part of a way to beat the seed. Found by dropping items
//!   one at a time while the seed stays beatable, from both ends of the
//!   playthrough; what's left either way is important.
//! - *Required for* a logic entry (e.g. a gossip stone): without it, that
//!   entry can't be reached.

use crate::{fill, prepare, reach, Outcome};
use mmr_logic::LogicFile;
use mmr_settings::Values;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Importance {
    /// Locations whose item is required.
    pub required: BTreeSet<String>,
    /// Locations whose item is important (required ones included).
    pub important: BTreeSet<String>,
    /// For each asked-about entry: the locations required to reach it, or
    /// `None` if it can't be reached at all.
    pub required_for: BTreeMap<String, Option<BTreeSet<String>>>,
}

/// Work out importance for a generated seed. `targets` are logic entries
/// to find requirements for.
pub fn importance(logic: &LogicFile, values: &Values, out: &Outcome, targets: &[String]) -> Importance {
    let game = mmr_data::game();
    let Ok(prepared) = prepare(logic, values, out.seed) else { return Importance::default() };
    let logic = &prepared.logic;
    let world = prepared.world(game, values);
    let graph = reach::Graph::new(&world, logic, values);
    let mut placement: fill::Placement = vec![None; world.items.len()];
    for (l, item) in world.items.iter().enumerate() {
        if item.location.is_some() && world.shuffled.binary_search(&l).is_err() {
            placement[l] = Some(l);
        }
    }
    for e in &out.layout {
        if let (Some(l), Some(i)) = (world.id(&e.location), world.id(&e.item)) {
            placement[l] = Some(i);
        }
    }
    let entry_index: BTreeMap<&str, usize> = logic.entries.iter().enumerate().map(|(i, e)| (e.id.as_str(), i)).collect();
    let base = graph.sweep(&world.starting, &placement, true);
    let mut result = Importance::default();
    let target_entries: Vec<(&String, Option<usize>)> = targets.iter().map(|t| (t, entry_index.get(t.as_str()).copied().filter(|&i| base.entries[i]))).collect();
    for (t, idx) in &target_entries {
        result.required_for.insert((*t).clone(), idx.map(|_| BTreeSet::new()));
    }

    // The items the playthrough collects, in order.
    let candidates: Vec<crate::world::Id> = out.spheres.iter().flatten().filter_map(|p| world.id(&p.location)).collect();
    for &l in &candidates {
        let saved = placement[l].take();
        let r = graph.sweep(&world.starting, &placement, true);
        let name = world.items[l].id.clone();
        if !graph.goal_reached(&r) {
            result.required.insert(name.clone());
        }
        for (t, idx) in &target_entries {
            if let Some(i) = *idx {
                if !r.entries[i] {
                    result.required_for.get_mut(*t).unwrap().as_mut().unwrap().insert(name.clone());
                }
            }
        }
        placement[l] = saved;
    }

    for order in [candidates.iter().rev().copied().collect::<Vec<_>>(), candidates.clone()] {
        let mut trial = placement.clone();
        let mut kept = Vec::new();
        for l in order {
            let saved = trial[l].take();
            if !graph.goal_reached(&graph.sweep(&world.starting, &trial, true)) {
                trial[l] = saved;
                kept.push(l);
            }
        }
        result.important.extend(kept.into_iter().map(|l| world.items[l].id.clone()));
    }
    result.important.extend(result.required.iter().cloned());
    result
}

