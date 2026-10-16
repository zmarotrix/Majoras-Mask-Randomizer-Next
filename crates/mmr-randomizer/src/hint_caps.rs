//! Hint priority caps (`OverrideHintItemCaps`, the old
//! `CheckAndUpdateHintedJunkLocations`): in each hint priority tier, at most
//! that many checks (or groups of checks hinted together) hold anything
//! worth having. Once a tier is full the rest of it becomes junk-only, and
//! after placement the extra groups of tiers that never filled up are
//! junked too (so hints don't point at them).

use crate::fill::Placement;
use crate::rng::Rng;
use crate::world::{Id, World};
use mmr_settings::Values;
use std::collections::{BTreeMap, BTreeSet};

pub struct HintCaps {
    /// (the tier's checks, how many may hold something real)
    tiers: Vec<(Vec<Id>, usize)>,
    /// Checks hinted together share a group.
    group: BTreeMap<Id, String>,
}

fn combinable() -> BTreeMap<String, Vec<String>> {
    serde_json::from_str(include_str!("../../../data/rom/combinable_hints.json").trim_start_matches('\u{FEFF}')).expect("bundled hint groups")
}

impl HintCaps {
    pub fn new(world: &World, values: &Values) -> Option<HintCaps> {
        let tiers: Vec<Vec<String>> = values
            .get("OverrideHintPriorities")?
            .as_array()?
            .iter()
            .map(|t| t.as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default())
            .collect();
        let caps: Vec<usize> = values.get("OverrideHintItemCaps")?.as_array()?.iter().map(|v| v.as_u64().unwrap_or(0) as usize).collect();
        let tiers: Vec<(Vec<Id>, usize)> = tiers
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let cap = caps.get(i).copied().unwrap_or(0);
                (cap > 0).then(|| (t.iter().filter_map(|l| world.id(l)).collect(), cap))
            })
            .collect();
        if tiers.is_empty() {
            return None;
        }
        let mut group = BTreeMap::new();
        for (name, locations) in combinable() {
            for l in locations {
                if let Some(id) = world.id(&l) {
                    group.insert(id, name.clone());
                }
            }
        }
        Some(HintCaps { tiers, group })
    }

    fn group_of(&self, world: &World, l: Id) -> String {
        self.group.get(&l).cloned().unwrap_or_else(|| world.items[l].id.clone())
    }

    fn real(world: &World, item: Id) -> bool {
        !world.is_junk(item) && !world.gives_junk.contains(&item)
    }

    /// Groups of a tier (its checks that aren't junk-only), and whether
    /// each holds something real.
    fn groups(&self, world: &World, placement: &Placement, tier: &[Id], capped: &BTreeSet<Id>) -> Vec<(String, Vec<Id>, bool)> {
        let mut out: Vec<(String, Vec<Id>, bool)> = Vec::new();
        for &l in tier.iter().filter(|l| !world.junk_locations.contains(l) && !capped.contains(l)) {
            let g = self.group_of(world, l);
            let real = placement[l].is_some_and(|i| Self::real(world, i));
            match out.iter_mut().find(|(name, _, _)| *name == g) {
                Some(e) => {
                    e.1.push(l);
                    e.2 |= real;
                }
                None => out.push((g, vec![l], real)),
            }
        }
        out
    }

    /// After something was placed at `location`: if its tier is now full,
    /// the rest of the tier becomes junk-only.
    pub fn after_place(&self, world: &World, placement: &Placement, location: Id, capped: &mut BTreeSet<Id>) {
        let Some(item) = placement[location] else { return };
        if !Self::real(world, item) {
            return;
        }
        let Some((tier, cap)) = self.tiers.iter().find(|(t, _)| t.contains(&location)) else { return };
        let groups = self.groups(world, placement, tier, capped);
        let full: Vec<&(String, Vec<Id>, bool)> = groups.iter().filter(|g| g.2).collect();
        if full.len() >= *cap {
            let keep: BTreeSet<Id> = full.iter().flat_map(|g| g.1.iter().copied()).collect();
            capped.extend(tier.iter().copied().filter(|l| !keep.contains(l)));
        }
    }

    /// After placement: tiers that never filled up have their extra groups
    /// (ones holding nothing real) junked, at random.
    pub fn finish(&self, world: &World, placement: &Placement, rng: &mut Rng, capped: &mut BTreeSet<Id>) {
        for (tier, cap) in &self.tiers {
            if tier.iter().any(|l| capped.contains(l)) {
                continue;
            }
            let groups = self.groups(world, placement, tier, capped);
            let extra = groups.len().saturating_sub(*cap);
            if extra == 0 {
                continue;
            }
            let empty: Vec<&(String, Vec<Id>, bool)> = groups.iter().filter(|g| !g.2).collect();
            for g in rng.sample(&empty, extra) {
                capped.extend(g.1.iter().copied());
            }
        }
    }
}
