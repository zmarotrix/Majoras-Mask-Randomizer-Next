//! Automatic conversion of v1 logic into regions.
//!
//! Works on any v1 logic file (bundled or community-made) with no hand-written
//! map. Places are entries that describe where you are (entrances, `Area...`
//! entries, `... Access` helpers, `Enter ...` helpers); everything else that is
//! tied to a place becomes an event in that place's region.
//!
//! A region's reachability is built from its place entry:
//! * **Required**: the entry requires another place, its parent; the exit
//!   from the parent keeps the rest of the rule.
//! * **Alternatives**: each alternative starts from a different place; one
//!   exit per alternative.
//! * **Inferred**: the entry mentions no place (e.g. `WFT Main Room Access`
//!   only says how to get from the temple entrance into the main room). The
//!   parent is inferred from where the entry is always used, which makes the
//!   region stricter than the old entry, so the old entry is kept.
//! * **Root**: no place at all; reachable from the start with its rule.
//!
//! Regions exactly equivalent to their old entry replace it, and references
//! are rewritten to the region. Check a conversion with `equivalence::compare`.

use crate::model::{Entry, EntryKind, LogicFile};
use crate::region::{region_entry_id, Exit, Region, RegionLogic, Rule, Spot};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const START: &str = "Start";

type Set = BTreeSet<String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParentKind {
    Required,
    Alternatives,
    Inferred,
    Root,
}

#[derive(Debug, Default, Serialize)]
pub struct AutoReport {
    pub regions: usize,
    pub equivalent_regions: usize,
    /// Places whose parent was inferred: (place, inferred parent).
    pub inferred: Vec<(String, String)>,
    pub locations_in_regions: usize,
    pub events_in_regions: usize,
    /// Checks whose rule names no place. They go into their game area's
    /// region, which v1 treats as reachable from the start.
    pub unmapped_locations: Vec<String>,
    /// Old place id -> region entry id, for every place replaced by a region.
    pub replaced: BTreeMap<String, String>,
}

/// Whether an entry describes a place.
pub fn is_place(e: &Entry) -> bool {
    if e.trick.is_some() || e.setting.is_some() {
        return false;
    }
    if let Some(item) = mmr_data::game().item(&e.id) {
        if item.location.is_some() {
            return false;
        }
        if item.entrance {
            return true;
        }
    }
    let id = e.id.as_str();
    id.starts_with("Area") || id.contains("Access") || id.starts_with("Enter ")
}

const ABBREVIATIONS: &[(&str, &str)] = &[
    ("ISTT", "Inverted Stone Tower Temple:"),
    ("WFT", "Woodfall Temple:"),
    ("SHT", "Snowhead Temple:"),
    ("GBT", "Great Bay Temple:"),
    ("STT", "Stone Tower Temple:"),
    ("OSH", "Ocean Spider House:"),
    ("SSH", "Swamp Spider House:"),
    ("PF", "Pirates' Fortress:"),
];

/// Names for places whose ids don't say what they are.
const NAMES: &[(&str, &str)] = &[
    ("AreaSouthAccess", "Southern Swamp: Past Big Octo"),
    ("AreaNorthAccess", "Mountain Village"),
    ("AreaWestAccess", "Great Bay Coast"),
    ("AreaEastAccess", "Ikana: Past the Ghost"),
    ("AreaIkanaCanyonAccess", "Ikana Canyon"),
    ("AreaSnowheadAccess", "Snowhead"),
    ("AreaMoonAccess", "The Moon"),
];

/// A readable region name from a place id: "AreaWoodFallTempleAccess" ->
/// "Woodfall Temple"; "WFT Main Room Access" -> "Woodfall Temple: Main Room".
pub fn region_name(id: &str) -> String {
    if let Some((_, n)) = NAMES.iter().find(|(k, _)| *k == id) {
        return n.to_string();
    }
    let mut s = id.to_string();
    if !s.contains(' ') {
        let body = s.trim_start_matches("Area");
        let mut out = String::new();
        for (i, c) in body.chars().enumerate() {
            if i > 0 && c.is_uppercase() && !out.ends_with(' ') {
                out.push(' ');
            }
            out.push(c);
        }
        s = out;
    }
    s = s.replace("Wood Fall", "Woodfall");
    s = s.strip_suffix(" Access").map(str::to_string).unwrap_or(s);
    s = s.strip_prefix("Access ").map(str::to_string).unwrap_or(s);
    s = s.strip_prefix("Enter ").map(str::to_string).unwrap_or(s);
    for (abbr, full) in ABBREVIATIONS {
        if let Some(rest) = s.strip_prefix(&format!("{abbr} ")) {
            s = format!("{full} {rest}");
            break;
        }
    }
    s.trim().to_string()
}

struct Place {
    kind: ParentKind,
    parent: Option<String>,
    equivalent: bool,
}

struct Ctx<'a> {
    entries: BTreeMap<&'a str, &'a Entry>,
    places: Set,
    implied: BTreeMap<String, Set>,
}

impl Ctx<'_> {
    fn is_place(&self, id: &str) -> bool {
        self.places.contains(id)
    }

    /// The place plus every place its rule requires, transitively.
    fn imp(&self, id: &str) -> Set {
        self.implied.get(id).cloned().unwrap_or_else(|| Set::from([id.to_string()]))
    }

    fn imp_all<'s>(&self, ids: impl IntoIterator<Item = &'s String>) -> Set {
        ids.into_iter().flat_map(|i| self.imp(i)).collect()
    }

    /// The most specific of several places (the one implying the most).
    fn most_specific<'s>(&self, ids: impl IntoIterator<Item = &'s String>) -> Option<String> {
        ids.into_iter().max_by_key(|i| (self.imp(i).len(), std::cmp::Reverse((*i).clone()))).cloned()
    }

    fn req_places(&self, e: &Entry) -> Vec<String> {
        e.requires.iter().filter(|r| self.is_place(r)).cloned().collect()
    }

    /// The place an alternative starts from: its most specific place.
    fn group_source(&self, group: &[String]) -> Option<String> {
        self.most_specific(group.iter().filter(|r| self.is_place(r)))
    }
}

/// Helpers that work as the way into an area: required by at least 3 checks,
/// at least 80% of them in the same game area (e.g. "Ocean Skulltulas" for
/// the Ocean Spider House).
fn gateway_helpers(v1: &LogicFile) -> Set {
    let game = mmr_data::game();
    let mut uses: BTreeMap<&str, Vec<Option<&str>>> = BTreeMap::new();
    for e in &v1.entries {
        let Some(item) = game.item(&e.id) else { continue };
        if item.location.is_none() {
            continue;
        }
        for r in &e.requires {
            uses.entry(r.as_str()).or_default().push(item.region.as_deref());
        }
    }
    let mut out = Set::new();
    for e in &v1.entries {
        if game.item(&e.id).is_some() || e.trick.is_some() || e.setting.is_some() || is_place(e) {
            continue;
        }
        let Some(regions) = uses.get(e.id.as_str()) else { continue };
        if regions.len() < 3 {
            continue;
        }
        let mut counts: BTreeMap<Option<&str>, usize> = BTreeMap::new();
        for r in regions {
            *counts.entry(*r).or_default() += 1;
        }
        let top = counts.values().max().copied().unwrap_or(0);
        if top * 5 >= regions.len() * 4 {
            out.insert(e.id.clone());
        }
    }
    out
}

fn compute_implied(entries: &BTreeMap<&str, &Entry>, places: &Set) -> BTreeMap<String, Set> {
    fn go(id: &str, entries: &BTreeMap<&str, &Entry>, places: &Set, memo: &mut BTreeMap<String, Set>, stack: &mut Vec<String>) -> Set {
        if let Some(s) = memo.get(id) {
            return s.clone();
        }
        let mut out = Set::from([id.to_string()]);
        if stack.iter().any(|s| s == id) {
            return out;
        }
        stack.push(id.to_string());
        if let Some(e) = entries.get(id) {
            for r in &e.requires {
                if places.contains(r) {
                    out.extend(go(r, entries, places, memo, stack));
                }
            }
        }
        stack.pop();
        memo.insert(id.to_string(), out.clone());
        out
    }
    let mut memo = BTreeMap::new();
    for p in places {
        go(p, entries, places, &mut memo, &mut Vec::new());
    }
    memo
}

pub fn to_regions_auto(v1: &LogicFile) -> (RegionLogic, AutoReport) {
    let game = mmr_data::game();
    let entries: BTreeMap<&str, &Entry> = v1.entries.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut places: Set = v1.entries.iter().filter(|e| is_place(e)).map(|e| e.id.clone()).collect();
    places.extend(gateway_helpers(v1));
    let implied = compute_implied(&entries, &places);
    let cx = Ctx { entries, places, implied };

    // For inferring parents: the places required alongside each place, by
    // checks and events (places built on top of a place say nothing about
    // where that place itself is).
    let mut required_with: BTreeMap<String, Vec<Set>> = BTreeMap::new();
    for e in v1.entries.iter().filter(|e| !cx.is_place(&e.id)) {
        let req = cx.req_places(e);
        for p in &req {
            let others: Vec<String> = req.iter().filter(|q| *q != p).cloned().collect();
            required_with.entry(p.clone()).or_default().push(cx.imp_all(&others));
        }
    }

    // Classify each place.
    let mut info: BTreeMap<String, Place> = BTreeMap::new();
    for id in &cx.places {
        let e = cx.entries[id.as_str()];
        let req = cx.req_places(e);
        let groups_with_place = e.any_of.iter().any(|g| g.iter().any(|r| cx.is_place(r)));
        let (kind, parent) = if !req.is_empty() {
            (ParentKind::Required, cx.most_specific(&req))
        } else if groups_with_place {
            (ParentKind::Alternatives, None)
        } else {
            let common: Set = required_with
                .get(id)
                .and_then(|uses| uses.iter().cloned().reduce(|a, b| a.intersection(&b).cloned().collect()))
                .unwrap_or_default();
            let candidates: Vec<String> = common.into_iter().filter(|q| q != id && !cx.imp(q).contains(id)).collect();
            match cx.most_specific(&candidates) {
                Some(p) => (ParentKind::Inferred, Some(p)),
                None => (ParentKind::Root, None),
            }
        };
        info.insert(id.clone(), Place { kind, parent, equivalent: kind != ParentKind::Inferred });
    }
    // Anything entered through a non-equivalent region isn't equivalent either.
    loop {
        let eq: BTreeMap<String, bool> = info.iter().map(|(k, p)| (k.clone(), p.equivalent)).collect();
        let mut changed = false;
        for (id, p) in info.iter_mut() {
            if !p.equivalent {
                continue;
            }
            let sources: Vec<String> = match p.kind {
                ParentKind::Required => p.parent.clone().into_iter().collect(),
                ParentKind::Alternatives => cx.entries[id.as_str()].any_of.iter().filter_map(|g| cx.group_source(g)).collect(),
                _ => vec![],
            };
            if sources.iter().any(|s| s == id || !eq.get(s).copied().unwrap_or(true)) {
                p.equivalent = false;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Unique names.
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    let mut used = Set::from([START.to_string()]);
    for id in &cx.places {
        let mut n = region_name(id);
        if used.contains(&n) {
            n = format!("{n} ({id})");
        }
        used.insert(n.clone());
        names.insert(id.clone(), n);
    }

    // References to equivalent places become references to their region.
    let replaced: BTreeMap<String, String> =
        info.iter().filter(|(_, p)| p.equivalent).map(|(id, _)| (id.clone(), region_entry_id(&names[id]))).collect();
    let rewrite = |ids: &[String]| -> Vec<String> { ids.iter().map(|r| replaced.get(r).cloned().unwrap_or_else(|| r.clone())).collect() };
    let rewrite_groups = |gs: &[Vec<String>]| -> Vec<Vec<String>> { gs.iter().map(|g| rewrite(g)).collect() };
    let rule = |req: &[String], groups: &[Vec<String>]| Rule::from_v1(&rewrite(req), &rewrite_groups(groups));

    let new_region = |name: &str| Region { name: name.into(), exits: vec![], locations: vec![], events: vec![], time: Default::default() };
    let mut regions: BTreeMap<String, Region> = BTreeMap::new();
    regions.insert(START.into(), new_region(START));
    for id in &cx.places {
        let mut r = new_region(&names[id]);
        r.time = cx.entries[id.as_str()].time;
        regions.insert(names[id].clone(), r);
    }
    let add_exit = |regions: &mut BTreeMap<String, Region>, from: &str, to: &str, rule: Rule| {
        regions.get_mut(from).expect("source region exists").exits.push(Exit { to: to.into(), rule });
    };
    // One region per game area. v1 treats these as reachable from the start
    // (their checks name no place), so they're entered from the start with
    // no requirement; the areas' own gates are the place regions.
    let area_region = |regions: &mut BTreeMap<String, Region>, area: &str| -> String {
        let mut name = game.region(area).map(|r| r.name.clone()).unwrap_or_else(|| area.to_string());
        if regions.contains_key(&name) && !regions[&name].exits.is_empty() || names.values().any(|n| *n == name) {
            name = format!("{name} (area)");
        }
        if !regions.contains_key(&name) {
            regions.insert(name.clone(), new_region(&name));
            regions.get_mut(START).unwrap().exits.push(Exit { to: name.clone(), rule: Rule::always() });
        }
        name
    };

    // Exits.
    for (id, p) in &info {
        let e = cx.entries[id.as_str()];
        let to = &names[id];
        match p.kind {
            ParentKind::Required | ParentKind::Inferred => {
                let parent = p.parent.as_ref().unwrap();
                let drop = if p.kind == ParentKind::Required { cx.imp(parent) } else { Set::new() };
                let req: Vec<String> = e.requires.iter().filter(|r| !drop.contains(*r)).cloned().collect();
                add_exit(&mut regions, &names[parent], to, rule(&req, &e.any_of));
            }
            ParentKind::Alternatives => {
                for g in &e.any_of {
                    let source = cx.group_source(g);
                    let drop = source.as_ref().map(|s| cx.imp(s)).unwrap_or_default();
                    let req: Vec<String> = e.requires.iter().filter(|r| !drop.contains(*r)).cloned().collect();
                    let rest: Vec<String> = g.iter().filter(|r| !drop.contains(*r)).cloned().collect();
                    let from = source.map(|s| names[&s].clone()).unwrap_or_else(|| START.to_string());
                    add_exit(&mut regions, &from, to, rule(&req, &[rest]));
                }
            }
            ParentKind::Root => {
                // Entrances and other places with a known game area are
                // entered from that area (a door in West Clock Town, say).
                let from = match game.item(id).and_then(|i| i.region.as_deref()) {
                    Some(area) => area_region(&mut regions, area),
                    None => START.to_string(),
                };
                add_exit(&mut regions, &from, to, rule(&e.requires, &e.any_of));
            }
        }
    }

    // What being in a region guarantees, as the places whose conjunction it
    // is exactly. Equivalent regions are just their place. A region reached
    // through an inferred parent also needs that parent's condition. A
    // non-equivalent region with several entrances has no such simple form,
    // so checks aren't moved into it (they keep their full rule elsewhere).
    let mut condition: BTreeMap<String, Option<Set>> = BTreeMap::new();
    fn cond(id: &str, info: &BTreeMap<String, Place>, memo: &mut BTreeMap<String, Option<Set>>, depth: usize) -> Option<Set> {
        if let Some(c) = memo.get(id) {
            return c.clone();
        }
        let p = &info[id];
        let c = if p.equivalent {
            Some(Set::from([id.to_string()]))
        } else if depth > 64 {
            None
        } else {
            match p.kind {
                ParentKind::Inferred | ParentKind::Required => {
                    cond(p.parent.as_deref().unwrap(), info, memo, depth + 1).map(|mut c| {
                        c.insert(id.to_string());
                        c
                    })
                }
                _ => None,
            }
        };
        memo.insert(id.to_string(), c.clone());
        c
    }
    for id in &cx.places {
        cond(id, &info, &mut condition, 0);
    }
    let condition: BTreeMap<String, Set> = condition.into_iter().filter_map(|(k, v)| v.map(|v| (k, v))).collect();

    // Move checks and events into the most specific region they require.
    let mut report = AutoReport { replaced: replaced.clone(), ..Default::default() };
    let mut moved = Set::new();
    for e in &v1.entries {
        if cx.is_place(&e.id) || crate::is_derived(&e.id) {
            continue;
        }
        let item = game.item(&e.id);
        let is_location = item.is_some_and(|i| i.location.is_some());
        let is_event = !is_location && matches!(EntryKind::of(e), EntryKind::Event | EntryKind::Helper);
        if !is_location && !is_event {
            continue;
        }
        let have = cx.imp_all(&cx.req_places(e));
        // Specificity: everything being in the region guarantees, i.e. the
        // places its condition implies (so an inferred child always ranks
        // above its parent).
        let guarantees = |q: &String| condition[q].iter().flat_map(|c| cx.imp(c)).collect::<Set>().len();
        let best = cx
            .places
            .iter()
            .filter(|q| condition.get(*q).is_some_and(|c| c.iter().all(|c| have.contains(c))))
            .max_by_key(|q| (guarantees(q), std::cmp::Reverse((*q).clone())))
            .cloned();
        let (region, drop) = match best {
            Some(q) => (names[&q].clone(), condition[&q].iter().flat_map(|c| cx.imp(c)).collect::<Set>()),
            None if is_location => {
                let area = item.and_then(|i| i.region.as_deref()).unwrap_or("Misc");
                report.unmapped_locations.push(e.id.clone());
                (area_region(&mut regions, area), Set::new())
            }
            None => continue, // an event not tied to any place stays a helper
        };
        let req: Vec<String> = e.requires.iter().filter(|r| !drop.contains(*r)).cloned().collect();
        let spot = Spot::from_entry(e, rule(&req, &e.any_of));
        let r = regions.get_mut(&region).unwrap();
        if is_location {
            r.locations.push(spot);
            report.locations_in_regions += 1;
        } else {
            r.events.push(spot);
            report.events_in_regions += 1;
        }
        moved.insert(e.id.clone());
    }

    // Remaining entries: helpers, tricks, settings, derived items, and places
    // that weren't replaced (kept for anything else that uses them).
    let rest: Vec<Entry> = v1
        .entries
        .iter()
        .filter(|e| !moved.contains(&e.id) && !replaced.contains_key(&e.id))
        .map(|e| Entry { requires: rewrite(&e.requires), any_of: rewrite_groups(&e.any_of), ..e.clone() })
        .collect();

    for r in regions.values_mut() {
        let mut merged: Vec<Exit> = Vec::new();
        for e in std::mem::take(&mut r.exits) {
            match merged.iter_mut().find(|m| m.to == e.to) {
                Some(m) => {
                    let mut any = match std::mem::replace(&mut m.rule, Rule::always()) {
                        Rule::Any { any } => any,
                        other => vec![other],
                    };
                    any.push(e.rule);
                    m.rule = Rule::Any { any };
                }
                None => merged.push(e),
            }
        }
        r.exits = merged;
    }

    report.regions = regions.len();
    report.equivalent_regions = replaced.len();
    report.inferred = info.iter().filter(|(_, p)| p.kind == ParentKind::Inferred).map(|(id, p)| (id.clone(), p.parent.clone().unwrap())).collect();

    let mut ordered = vec![regions.remove(START).unwrap()];
    ordered.extend(regions.into_values());
    let logic = RegionLogic { format: crate::model::FORMAT_NAME.into(), version: 2, start: START.into(), regions: ordered, entries: rest };
    (logic, report)
}
