//! Convert v1 logic to regions.
//!
//! v1 has no map, but its "place" entries (e.g. `AreaWoodFallTempleAccess`,
//! `WFT Main Room Access`) are regions in all but name. A [`RegionSpec`] list
//! says which place entry defines each region and where it's entered from;
//! the rest is mechanical:
//!
//! * each region is entered from its parent by an exit whose rule is the
//!   place entry's rule, minus the parent places it repeats;
//! * each check moves into the deepest region whose places its rule
//!   requires, keeping only the rest as its local rule;
//! * checks without such a place go to the region they're physically in,
//!   with their full rule.
//!
//! `equivalence::compare` then proves the result behaves exactly the same.

use crate::model::{Entry, LogicFile};
use crate::region::{Exit, Region, RegionLogic, Rule, Spot};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct RegionSpec {
    pub name: &'static str,
    /// Region it's entered from (`None` for the start).
    pub parent: Option<&'static str>,
    /// The v1 place entry this region stands for.
    pub place: Option<&'static str>,
    /// Use the place entry itself as the exit rule instead of its contents
    /// (for places whose logic lives outside the converted area).
    pub exit_by_reference: bool,
    /// Game-data regions whose checks belong here when no place says otherwise.
    pub home: &'static [&'static str],
}

const fn region(name: &'static str, parent: Option<&'static str>, place: Option<&'static str>, home: &'static [&'static str]) -> RegionSpec {
    RegionSpec { name, parent, place, exit_by_reference: false, home }
}

/// Southern Swamp and Woodfall Temple, the proof-of-concept area.
pub fn swamp_spec() -> Vec<RegionSpec> {
    vec![
        region("Termina Field", None, None, &[]),
        region("Road to Southern Swamp", Some("Termina Field"), None, &["RoadToSouthernSwamp"]),
        region("Southern Swamp", Some("Road to Southern Swamp"), None, &["SouthernSwamp"]),
        RegionSpec { exit_by_reference: true, ..region("Southern Swamp (Clear)", Some("Southern Swamp"), Some("AreaWoodFallTempleClear"), &[]) },
        region("Deep Southern Swamp", Some("Southern Swamp"), Some("AreaSouthAccess"), &[]),
        region("Woodfall", Some("Deep Southern Swamp"), None, &["Woodfall"]),
        region("Woodfall Temple", Some("Woodfall"), Some("AreaWoodFallTempleAccess"), &["WoodfallTemple"]),
        region("Woodfall Temple: Main Room", Some("Woodfall Temple"), Some("WFT Main Room Access"), &[]),
        region("Woodfall Temple: Past Locked Door", Some("Woodfall Temple: Main Room"), Some("WFT Past Locked Door"), &[]),
        region("Woodfall Temple: 2nd Floor", Some("Woodfall Temple: Main Room"), Some("WFT 2nd Floor Access"), &[]),
        region("Woodfall Temple: Pre-Boss Room", Some("Woodfall Temple: 2nd Floor"), Some("WFT Pre-Boss Room Access"), &[]),
    ]
}

#[derive(Debug, Default)]
pub struct Report {
    /// Checks moved into regions: (check, region).
    pub moved: Vec<(String, String)>,
}

/// Convert the checks in `spec`'s area to regions. Everything else stays as
/// v1 entries.
pub fn to_regions(v1: &LogicFile, spec: &[RegionSpec]) -> (RegionLogic, Report) {
    let game = mmr_data::game();
    let by_name: BTreeMap<&str, &RegionSpec> = spec.iter().map(|s| (s.name, s)).collect();

    // Places along the path to each region (its own and its ancestors').
    let closure = |name: &str| -> Vec<&'static str> {
        let mut out = Vec::new();
        let mut cur = by_name.get(name).copied();
        while let Some(s) = cur {
            if let Some(p) = s.place {
                out.push(p);
            }
            cur = s.parent.and_then(|p| by_name.get(p).copied());
        }
        out
    };
    let depth = |name: &str| -> usize {
        let mut d = 0;
        let mut cur = by_name.get(name).copied();
        while let Some(s) = cur.and_then(|s| s.parent).and_then(|p| by_name.get(p).copied()) {
            d += 1;
            cur = Some(s);
        }
        d
    };

    let mut regions: Vec<Region> = spec.iter().map(|s| Region { name: s.name.into(), exits: vec![], locations: vec![], events: vec![], time: Default::default() }).collect();
    let index: BTreeMap<&str, usize> = spec.iter().enumerate().map(|(i, s)| (s.name, i)).collect();

    // Exits.
    for s in spec {
        let Some(parent) = s.parent else { continue };
        let rule = match s.place {
            None => Rule::always(),
            Some(p) if s.exit_by_reference => Rule::Ref(p.into()),
            Some(p) => {
                let e = v1.entry(p).expect("place entry exists");
                let inherited: BTreeSet<&str> = closure(parent).into_iter().collect();
                let requires: Vec<String> = e.requires.iter().filter(|r| !inherited.contains(r.as_str())).cloned().collect();
                Rule::from_v1(&requires, &e.any_of)
            }
        };
        regions[index[parent]].exits.push(Exit { to: s.name.into(), rule });
    }

    // Checks.
    let places: BTreeSet<&str> = spec.iter().filter_map(|s| s.place).collect();
    let home: BTreeMap<&str, &str> = spec.iter().flat_map(|s| s.home.iter().map(move |h| (*h, s.name))).collect();
    let mut report = Report::default();
    let mut moved: BTreeSet<&str> = BTreeSet::new();
    for e in &v1.entries {
        let Some(item) = game.item(&e.id) else { continue };
        let Some(home_region) = item.region.as_deref().and_then(|r| home.get(r)).copied() else { continue };
        if item.location.is_none() || crate::is_derived(&e.id) {
            continue;
        }
        // Places this check needs, including places those places need
        // (temple access implies deep-swamp access, and so on).
        let mut required: BTreeSet<&str> = BTreeSet::new();
        let mut todo: Vec<&str> = e.requires.iter().map(String::as_str).filter(|r| places.contains(r)).collect();
        while let Some(p) = todo.pop() {
            if required.insert(p) {
                if let Some(pe) = v1.entry(p) {
                    todo.extend(pe.requires.iter().map(String::as_str).filter(|r| places.contains(r)));
                }
            }
        }
        // The most specific region whose places are all needed; on a tie,
        // the region the check is physically in, then the deeper one.
        let mut candidates: Vec<&str> = spec.iter().map(|s| s.name).filter(|n| closure(n).iter().all(|p| required.contains(p))).collect();
        candidates.sort_by_key(|n| (std::cmp::Reverse(closure(n).len()), *n != home_region, std::cmp::Reverse(depth(n))));
        let best = candidates[0];
        let implied: BTreeSet<&str> = closure(best).into_iter().collect();
        let requires: Vec<String> = e.requires.iter().filter(|r| !implied.contains(r.as_str())).cloned().collect();
        regions[index[best]].locations.push(Spot::from_entry(e, Rule::from_v1(&requires, &e.any_of)));
        report.moved.push((e.id.clone(), best.to_string()));
        moved.insert(e.id.as_str());
    }

    let entries: Vec<Entry> = v1.entries.iter().filter(|e| !moved.contains(e.id.as_str())).cloned().collect();
    let logic = RegionLogic { format: crate::model::FORMAT_NAME.into(), version: 2, start: spec[0].name.into(), regions, entries };
    (logic, report)
}
