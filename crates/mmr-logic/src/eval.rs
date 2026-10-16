//! Evaluating logic: what is reachable with a given inventory, and a
//! playthrough of the unrandomized game.
//!
//! Time of day is not simulated here; these answer "is this possible with
//! these items", which is what the editor's test panel needs. The item
//! placement algorithm does its own, time-aware checking (see
//! `mmr_randomizer::reach`).

use crate::model::{EntryKind, LogicFile};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

/// The player's situation to evaluate logic against.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scenario {
    /// Items the player has (item ids).
    #[serde(default)]
    pub items: HashSet<String>,
    /// Tricks the player is willing to do (entry ids).
    #[serde(default)]
    pub tricks: HashSet<String>,
    /// Setting values, keyed by setting key.
    #[serde(default)]
    pub settings: Map<String, Value>,
}

/// Which entries are satisfied. References to ids with no entry count as
/// satisfied (validation reports them as errors).
#[derive(Debug, Clone, Default, Serialize)]
pub struct Evaluation {
    pub satisfied: HashSet<String>,
}

impl Evaluation {
    pub fn is_satisfied(&self, id: &str) -> bool {
        self.satisfied.contains(id)
    }
}

/// Precomputed view of a logic file for repeated evaluation.
pub struct Evaluator<'a> {
    file: &'a LogicFile,
    kinds: Vec<EntryKind>,
    /// For each entry, its references with `true` if it's an item reference.
    refs: Vec<(Vec<(&'a str, bool)>, Vec<Vec<(&'a str, bool)>>)>,
}

impl<'a> Evaluator<'a> {
    pub fn new(file: &'a LogicFile) -> Self {
        let tag = |r: &'a String| (r.as_str(), crate::is_item_ref(r));
        Evaluator {
            file,
            kinds: file.entries.iter().map(EntryKind::of).collect(),
            refs: file
                .entries
                .iter()
                // `temporary_requires` count: without time, assume the worst.
                .map(|e| {
                    (
                        e.requires.iter().chain(&e.temporary_requires).map(tag).collect(),
                        e.any_of.iter().map(|g| g.iter().map(tag).collect()).collect(),
                    )
                })
                .collect(),
        }
    }

    pub fn evaluate(&self, scenario: &Scenario) -> Evaluation {
        let ids: HashSet<&str> = self.file.entries.iter().map(|e| e.id.as_str()).collect();
        let mut satisfied: HashSet<&str> = HashSet::new();
        // Fixed point: keep marking entries until nothing changes.
        loop {
            let mut changed = false;
            for (i, e) in self.file.entries.iter().enumerate() {
                if satisfied.contains(e.id.as_str()) {
                    continue;
                }
                let ok = |(r, is_item): &(&str, bool)| {
                    if *is_item {
                        scenario.items.contains(*r)
                    } else {
                        *r != crate::settings_rules::IMPOSSIBLE && (satisfied.contains(r) || !ids.contains(r))
                    }
                };
                let gate = match self.kinds[i] {
                    EntryKind::Trick => scenario.tricks.contains(&e.id),
                    EntryKind::Setting => e.setting.as_ref().is_some_and(|c| c.eval(&scenario.settings)),
                    _ => true,
                };
                let (req, groups) = &self.refs[i];
                if gate && req.iter().all(ok) && (groups.is_empty() || groups.iter().any(|g| g.iter().all(ok))) {
                    satisfied.insert(e.id.as_str());
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Evaluation { satisfied: satisfied.into_iter().map(String::from).collect() }
    }

    /// Play the unrandomized game: repeatedly collect every reachable
    /// location's vanilla item, starting with `start_items`.
    pub fn vanilla_playthrough(&self, scenario: &Scenario) -> Playthrough {
        let game = mmr_data::game();
        let mut scenario = scenario.clone();
        let mut sphere_of: HashMap<String, usize> = HashMap::new();
        let mut spheres: Vec<Vec<String>> = Vec::new();
        loop {
            let eval = self.evaluate(&scenario);
            let mut new: Vec<String> = self
                .file
                .entries
                .iter()
                .filter(|e| eval.is_satisfied(&e.id) && !sphere_of.contains_key(&e.id))
                .filter(|e| game.item(&e.id).is_some_and(|i| !i.is_fake()))
                .map(|e| e.id.clone())
                .collect();
            if new.is_empty() {
                break;
            }
            new.sort_by_key(|id| game.item(id).map(|i| i.index));
            for id in &new {
                sphere_of.insert(id.clone(), spheres.len());
                scenario.items.insert(id.clone());
            }
            spheres.push(new);
        }
        let final_eval = self.evaluate(&scenario);
        let unreachable = game
            .locations()
            .filter(|i| !i.is_fake() && self.file.entry(&i.id).is_some() && !sphere_of.contains_key(&i.id))
            .map(|i| i.id.clone())
            .collect();
        Playthrough { beatable: final_eval.is_satisfied(crate::GOAL), spheres, unreachable }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Playthrough {
    /// Whether the goal (the credits) is reachable.
    pub beatable: bool,
    /// Locations in the order they become reachable; sphere 0 needs nothing.
    pub spheres: Vec<Vec<String>>,
    /// Locations that can never be reached.
    pub unreachable: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Entry, Trick};

    #[test]
    fn requires_and_alternatives() {
        let mut bow = Entry::new("ItemBow");
        bow.requires = vec!["Area".into()];
        bow.any_of = vec![vec!["MaskDeku".into(), "Magic".into()], vec!["MaskGoron".into()]];
        let file = LogicFile::new(vec![bow, Entry::new("Area"), Entry::new("Magic")]);
        let ev = Evaluator::new(&file);

        let mut s = Scenario::default();
        assert!(!ev.evaluate(&s).is_satisfied("ItemBow"));
        s.items.insert("MaskGoron".into());
        assert!(ev.evaluate(&s).is_satisfied("ItemBow"));
        s.items.clear();
        s.items.insert("MaskDeku".into());
        assert!(ev.evaluate(&s).is_satisfied("ItemBow"), "helper Magic has no requirements");
    }

    #[test]
    fn tricks_and_settings_gate_entries() {
        let mut t = Entry::new("Lensless Chests");
        t.trick = Some(Trick::default());
        let mut c = Entry::new("SettingCloseCows");
        c.setting = Some(crate::SettingCondition::from_legacy("settings.CloseCows").unwrap());
        let file = LogicFile::new(vec![t, c]);
        let ev = Evaluator::new(&file);
        let mut s = Scenario::default();
        assert!(ev.evaluate(&s).satisfied.is_empty());
        s.tricks.insert("Lensless Chests".into());
        s.settings.insert("CloseCows".into(), true.into());
        assert_eq!(ev.evaluate(&s).satisfied.len(), 2);
    }

    #[test]
    fn vanilla_casual_is_beatable() {
        let file = crate::bundled("casual").unwrap();
        let ev = Evaluator::new(&file);
        let p = ev.vanilla_playthrough(&Scenario::default());
        assert!(p.beatable, "unreachable: {:?}", &p.unreachable[..p.unreachable.len().min(20)]);
        assert!(p.spheres.len() > 3);
    }
}
