//! Region-graph logic (format version 2, proof of concept).
//!
//! Logic is a map: regions connected by exits. Each check and event says only
//! what it needs once you're standing in its region. Entrance randomization
//! then just rewires exits; every check's logic stays correct.
//!
//! Regions compile down to the flat entry form ([`LogicFile`]) that the
//! evaluator and randomizer already run, which also makes old and new logic
//! directly comparable (see `equivalence`).

use crate::model::{Entry, LogicFile, Time, TimeSet};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A requirement. Leaves are ids, resolved as in v1: an item you can own
/// means "has the item", anything else means "that entry is satisfied".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Rule {
    Ref(String),
    All { all: Vec<Rule> },
    Any { any: Vec<Rule> },
    /// At least `count` of the listed ids.
    Count { count: usize, of: Vec<String> },
}

impl Rule {
    pub fn always() -> Rule {
        Rule::All { all: vec![] }
    }

    pub fn is_always(&self) -> bool {
        matches!(self, Rule::All { all } if all.is_empty())
    }

    /// From the v1 shape: everything in `requires`, and one `any_of` group.
    pub fn from_v1(requires: &[String], any_of: &[Vec<String>]) -> Rule {
        let mut all: Vec<Rule> = requires.iter().cloned().map(Rule::Ref).collect();
        match any_of.len() {
            0 => {}
            1 => all.extend(any_of[0].iter().cloned().map(Rule::Ref)),
            _ => all.push(Rule::Any {
                any: any_of
                    .iter()
                    .map(|g| if g.len() == 1 { Rule::Ref(g[0].clone()) } else { Rule::All { all: g.iter().cloned().map(Rule::Ref).collect() } })
                    .collect(),
            }),
        }
        if all.len() == 1 {
            all.pop().unwrap()
        } else {
            Rule::All { all }
        }
    }

    /// Disjunctive normal form: a list of alternatives, each a list of ids
    /// that are all needed. `[]` means impossible; `[[]]` means always.
    pub fn dnf(&self) -> Vec<Vec<String>> {
        match self {
            Rule::Ref(id) => vec![vec![id.clone()]],
            Rule::Any { any } => any.iter().flat_map(Rule::dnf).collect(),
            Rule::All { all } => {
                let mut acc: Vec<Vec<String>> = vec![vec![]];
                for r in all {
                    let d = r.dnf();
                    acc = acc.iter().flat_map(|a| d.iter().map(move |b| merge(a, b))).collect();
                }
                acc
            }
            Rule::Count { count, of } => combinations(of, *count),
        }
    }
}

fn merge(a: &[String], b: &[String]) -> Vec<String> {
    let mut v = a.to_vec();
    for x in b {
        if !v.contains(x) {
            v.push(x.clone());
        }
    }
    v
}

fn combinations(items: &[String], k: usize) -> Vec<Vec<String>> {
    if k == 0 {
        return vec![vec![]];
    }
    if items.len() < k {
        return vec![];
    }
    let mut with: Vec<Vec<String>> = combinations(&items[1..], k - 1)
        .into_iter()
        .map(|mut c| {
            c.insert(0, items[0].clone());
            c
        })
        .collect();
    with.extend(combinations(&items[1..], k));
    with
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Exit {
    pub to: String,
    #[serde(default = "Rule::always", skip_serializing_if = "Rule::is_always")]
    pub rule: Rule,
}

/// A check or event inside a region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spot {
    /// Location id (for checks) or event name.
    pub id: String,
    #[serde(default = "Rule::always", skip_serializing_if = "Rule::is_always")]
    pub rule: Rule,
    #[serde(default, skip_serializing_if = "Time::is_empty")]
    pub time: Time,
    /// See [`Entry::before`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<String>,
    /// See [`Entry::temporary_requires`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub temporary_requires: Vec<String>,
}

impl Spot {
    pub fn new(id: impl Into<String>, rule: Rule, time: Time) -> Spot {
        Spot { id: id.into(), rule, time, before: Vec::new(), temporary_requires: Vec::new() }
    }

    /// A spot from a v1 entry, keeping everything but the rule.
    pub fn from_entry(e: &Entry, rule: Rule) -> Spot {
        Spot { id: e.id.clone(), rule, time: e.time, before: e.before.clone(), temporary_requires: e.temporary_requires.clone() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exits: Vec<Exit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<Spot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<Spot>,
    /// Time-of-day constraints carried over from the v1 place entry.
    #[serde(default, skip_serializing_if = "Time::is_empty")]
    pub time: Time,
}

/// A logic file with regions. `entries` holds logic not yet converted to
/// regions (helpers, tricks, settings, and areas still in the old style).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionLogic {
    pub format: String,
    pub version: u32,
    /// The region the player starts in.
    pub start: String,
    pub regions: Vec<Region>,
    pub entries: Vec<Entry>,
}

/// Id of the compiled entry meaning "can reach this region".
pub fn region_entry_id(name: &str) -> String {
    format!("Region: {name}")
}

impl RegionLogic {
    /// Compile to flat entries the evaluator runs. Each region becomes an
    /// entry satisfied by any exit into it; each check/event requires its
    /// region plus its own rule.
    pub fn compile(&self) -> LogicFile {
        let mut entries: Vec<Entry> = Vec::new();
        let mut into: BTreeMap<&str, Vec<Vec<String>>> = BTreeMap::new();
        into.entry(self.start.as_str()).or_default().push(vec![]);
        for r in &self.regions {
            for e in &r.exits {
                let from = region_entry_id(&r.name);
                for alt in e.rule.dnf() {
                    into.entry(e.to.as_str()).or_default().push(merge(&[from.clone()], &alt));
                }
            }
        }
        for r in &self.regions {
            let mut entry = Entry::new(region_entry_id(&r.name));
            entry.time = r.time;
            entry.any_of = into.remove(r.name.as_str()).unwrap_or_default();
            if entry.any_of.iter().any(|g| g.is_empty()) {
                entry.any_of.clear(); // always reachable
            } else if entry.any_of.is_empty() {
                entry.any_of = vec![vec![region_entry_id("(unreachable)")]];
            }
            entries.push(entry);
            for spot in r.locations.iter().chain(&r.events) {
                entries.push(spot_entry(&r.name, spot));
            }
        }
        entries.extend(self.entries.iter().cloned());
        LogicFile::new(entries)
    }
}

fn spot_entry(region: &str, spot: &Spot) -> Entry {
    let alts = spot.rule.dnf();
    let mut e = Entry::new(spot.id.clone());
    // Factor out what every alternative needs, like v1 `requires`.
    let common: Vec<String> = alts.first().map(|f| f.iter().filter(|x| alts.iter().all(|a| a.contains(x))).cloned().collect()).unwrap_or_default();
    e.requires = merge(&[region_entry_id(region)], &common);
    let rest: Vec<Vec<String>> = alts.iter().map(|a| a.iter().filter(|x| !common.contains(x)).cloned().collect()).collect();
    if !rest.iter().any(|g: &Vec<String>| g.is_empty()) {
        e.any_of = rest;
    }
    if alts.is_empty() {
        e.requires.push(region_entry_id("(unreachable)"));
    }
    e.time = spot.time;
    e.before = spot.before.clone();
    e.temporary_requires = spot.temporary_requires.clone();
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(s: &str) -> Rule {
        Rule::Ref(s.into())
    }

    #[test]
    fn dnf_and_counts() {
        let rule = Rule::All { all: vec![r("A"), Rule::Any { any: vec![r("B"), r("C")] }] };
        assert_eq!(rule.dnf(), vec![vec!["A", "B"], vec!["A", "C"]]);
        let keys = Rule::Count { count: 2, of: vec!["K1".into(), "K2".into(), "K3".into()] };
        assert_eq!(keys.dnf().len(), 3);
        assert_eq!(Rule::always().dnf(), vec![Vec::<String>::new()]);
    }

    #[test]
    fn compiles_regions_to_entries() {
        let logic = RegionLogic {
            format: "mmr-logic".into(),
            version: 2,
            start: "Field".into(),
            regions: vec![
                Region { name: "Field".into(), exits: vec![Exit { to: "Swamp".into(), rule: r("Boat") }], locations: vec![], events: vec![], time: Time::default() },
                Region {
                    name: "Swamp".into(),
                    exits: vec![],
                    locations: vec![Spot::new("Chest", Rule::Any { any: vec![r("Bow"), r("Hookshot")] }, Time::default())],
                    events: vec![],
                    time: Time::default(),
                },
            ],
            entries: vec![],
        };
        let flat = logic.compile();
        let chest = flat.entry("Chest").unwrap();
        assert_eq!(chest.requires, vec!["Region: Swamp"]);
        assert_eq!(chest.any_of, vec![vec!["Bow"], vec!["Hookshot"]]);
        assert_eq!(flat.entry("Region: Swamp").unwrap().any_of, vec![vec!["Region: Field", "Boat"]]);
        assert!(flat.entry("Region: Field").unwrap().any_of.is_empty());
    }
}

/// Human-readable text form, e.g.
///
/// ```text
/// region "Woodfall Temple: 2nd Floor"
///   locations
///     Hero's Bow Chest: Any Sword or Arrows or (Deku Mask and Magic Meter)
/// ```
pub mod text {
    use super::*;

    /// Display name for an id: item name for items, location name for
    /// checks (when `as_location`), otherwise the id itself.
    pub fn name(id: &str, as_location: bool) -> String {
        let game = mmr_data::game();
        match game.item(id) {
            Some(i) if as_location && i.location.is_some() => i.location.clone().unwrap(),
            Some(i) if i.name.is_some() => i.name.clone().unwrap(),
            _ => id.to_string(),
        }
    }

    pub fn rule(r: &Rule) -> String {
        fn go(r: &Rule, top: bool) -> String {
            match r {
                Rule::Ref(id) => name(id, false),
                Rule::Count { count, of } => format!("{count} of ({})", of.iter().map(|i| name(i, false)).collect::<Vec<_>>().join(", ")),
                Rule::All { all } if all.is_empty() => "always".into(),
                Rule::All { all } => {
                    let s = all.iter().map(|x| go(x, false)).collect::<Vec<_>>().join(" and ");
                    if top || all.len() == 1 { s } else { format!("({s})") }
                }
                Rule::Any { any } => {
                    let s = any.iter().map(|x| go(x, false)).collect::<Vec<_>>().join(" or ");
                    if top || any.len() == 1 { s } else { format!("({s})") }
                }
            }
        }
        go(r, true)
    }

    /// Time rules of a spot, e.g. ` [set up Day1; available Night3]`.
    pub fn notes(spot: &Spot) -> String {
        let t = &spot.time;
        let times = |s: TimeSet| s.names().join(", ");
        let names = |ids: &[String], sep: &str| ids.iter().map(|b| name(b, false)).collect::<Vec<_>>().join(sep);
        let mut parts = Vec::new();
        if !t.setup.is_empty() {
            parts.push(format!("set up {}", times(t.setup)));
        }
        if !t.available.is_empty() {
            parts.push(format!("available {}", times(t.available)));
        }
        if !t.needed.is_empty() {
            parts.push(format!("needed {}", times(t.needed)));
        }
        if t.no_return {
            parts.push("no return".into());
        }
        if t.opening_hours {
            parts.push("opening hours".into());
        }
        if !spot.before.is_empty() {
            parts.push(format!("before {}", names(&spot.before, ", ")));
        }
        if !spot.temporary_requires.is_empty() {
            parts.push(format!("if not kept also {}", names(&spot.temporary_requires, " and ")));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(" [{}]", parts.join("; "))
        }
    }

    pub fn regions(regions: &[Region]) -> String {
        let mut out = String::new();
        for r in regions {
            out += &format!("region \"{}\"\n", r.name);
            if !r.exits.is_empty() {
                out += "  exits\n";
                for e in &r.exits {
                    out += &format!("    -> \"{}\": {}\n", e.to, rule(&e.rule));
                }
            }
            if !r.locations.is_empty() {
                out += "  locations\n";
                for l in &r.locations {
                    out += &format!("    {}: {}{}\n", name(&l.id, true), rule(&l.rule), notes(l));
                }
            }
            out += "\n";
        }
        out
    }
}
