//! Problems in a logic file, for display in the editor and CLI.

use crate::model::{EntryKind, LogicFile};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The randomizer cannot use this logic.
    Error,
    /// Probably a mistake.
    Warning,
    /// Harmless, but could be tidied up.
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub severity: Severity,
    /// Stable machine-readable code, e.g. `unknown_reference`.
    pub code: &'static str,
    /// The entry the issue is about, if any.
    pub entry: Option<String>,
    /// The referenced id involved, if any (for "go to" links).
    pub target: Option<String>,
    pub message: String,
}

impl Issue {
    pub fn new(severity: Severity, code: &'static str, entry: &str, message: String) -> Self {
        Issue { severity, code, entry: Some(entry.to_string()), target: None, message }
    }

    fn with_target(mut self, target: &str) -> Self {
        self.target = Some(target.to_string());
        self
    }
}

/// Validate a logic file. `known_settings`, if given, is the set of valid
/// setting keys; settings conditions are checked against it.
pub fn validate(file: &LogicFile, known_settings: Option<&HashSet<String>>) -> Vec<Issue> {
    let mut issues = Vec::new();
    let game = mmr_data::game();

    let mut seen = HashSet::new();
    for e in &file.entries {
        if !seen.insert(e.id.as_str()) {
            issues.push(Issue::new(Severity::Error, "duplicate_id", &e.id, format!("There is more than one entry named \"{}\".", e.id)));
        }
    }
    let ids: HashSet<&str> = file.entries.iter().map(|e| e.id.as_str()).collect();

    for item in &game.items {
        let used_by_randomizer = !item.is_fake() || item.is_location();
        if used_by_randomizer && !ids.contains(item.id.as_str()) && item.multi_location.is_none() && item.name.is_some() && item.is_location() {
            issues.push(Issue {
                severity: Severity::Warning,
                code: "missing_location",
                entry: None,
                target: Some(item.id.clone()),
                message: format!("No entry for the location \"{}\". It will be treated as always reachable.", item.location.as_deref().unwrap_or(&item.id)),
            });
        }
    }

    for e in &file.entries {
        let derived = crate::is_derived(&e.id);
        // Referring to your own id is fine for real items and entrances: the
        // location can require the item (or entrance), which may be shuffled
        // elsewhere.
        if !crate::is_item_ref(&e.id) && !game.item(&e.id).is_some_and(|i| i.entrance) {
            if e.requires.contains(&e.id) {
                issues.push(Issue::new(Severity::Error, "self_reference", &e.id, "This entry requires itself, so it can never be satisfied.".into()));
            }
            for (i, group) in e.any_of.iter().enumerate() {
                if group.contains(&e.id) {
                    issues.push(Issue::new(
                        Severity::Warning,
                        "self_reference",
                        &e.id,
                        format!("Alternative {} requires this entry itself, so it can never be used.", i + 1),
                    ));
                }
            }
        }
        for r in e.references() {
            if r != e.id && !ids.contains(r) && game.item(r).is_none() {
                issues.push(Issue::new(Severity::Error, "unknown_reference", &e.id, format!("Refers to \"{r}\", which doesn't exist.")).with_target(r));
            }
        }
        if derived {
            continue;
        }

        let mut dupes = HashSet::new();
        for r in &e.requires {
            if !dupes.insert(r) {
                issues.push(Issue::new(Severity::Info, "duplicate_requirement", &e.id, format!("\"{r}\" is listed twice under required.")).with_target(r));
            }
        }
        for (i, group) in e.any_of.iter().enumerate() {
            if group.is_empty() {
                issues.push(Issue::new(
                    Severity::Warning,
                    "empty_group",
                    &e.id,
                    format!("Alternative {} is empty, so it is always satisfied and the other alternatives don't matter.", i + 1),
                ));
            }
            let set: HashSet<&String> = group.iter().collect();
            if set.len() != group.len() {
                issues.push(Issue::new(Severity::Info, "duplicate_requirement", &e.id, format!("Alternative {} lists the same thing twice.", i + 1)));
            }
            if let Some(r) = group.iter().find(|r| e.requires.contains(r)) {
                issues.push(
                    Issue::new(Severity::Info, "redundant_requirement", &e.id, format!("\"{r}\" is already required, so listing it in alternative {} has no effect.", i + 1))
                        .with_target(r),
                );
            }
            for (j, other) in e.any_of.iter().enumerate() {
                if i != j && !other.is_empty() && other.len() < group.len() && other.iter().all(|r| set.contains(r)) {
                    issues.push(Issue::new(
                        Severity::Info,
                        "redundant_group",
                        &e.id,
                        format!("Alternative {} needs everything in alternative {} and more, so it is never needed.", i + 1, j + 1),
                    ));
                    break;
                }
            }
        }
        if let Some(t) = &e.trick {
            if t.description.as_deref().is_none_or(|d| d.trim().is_empty()) {
                issues.push(Issue::new(Severity::Warning, "trick_without_description", &e.id, "This trick has no description, so players won't know what it involves.".into()));
            }
        }
        if let (Some(cond), Some(known)) = (&e.setting, known_settings) {
            let key = cond.setting_key();
            if !known.contains(key) {
                issues.push(Issue::new(Severity::Error, "unknown_setting", &e.id, format!("Depends on the setting \"{key}\", which doesn't exist.")));
            }
        }
        if e.trick.is_some() && e.setting.is_some() {
            issues.push(Issue::new(Severity::Warning, "trick_and_setting", &e.id, "This entry is marked both as a trick and as a setting; it will be treated as a trick.".into()));
        }
    }

    issues.extend(hard_cycles(file));
    issues.sort_by(|a, b| a.severity.cmp(&b.severity));
    issues
}

/// Cycles through *hard* requirements (`requires`, or the only alternative)
/// between non-item entries. Such entries can never be satisfied.
fn hard_cycles(file: &LogicFile) -> Vec<Issue> {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &file.entries {
        if matches!(EntryKind::of(e), EntryKind::Trick | EntryKind::Setting) {
            continue;
        }
        let mut hard: Vec<&str> = e.requires.iter().map(String::as_str).collect();
        if e.any_of.len() == 1 {
            hard.extend(e.any_of[0].iter().map(String::as_str));
        }
        hard.retain(|r| !crate::is_item_ref(r) && *r != e.id);
        graph.insert(e.id.as_str(), hard);
    }

    // Iterative DFS with colors; report each cycle once.
    let mut state: HashMap<&str, u8> = HashMap::new(); // 1 = on stack, 2 = done
    let mut issues = Vec::new();
    let mut reported: HashSet<Vec<&str>> = HashSet::new();
    for &start in graph.keys() {
        if state.contains_key(start) {
            continue;
        }
        let mut stack: Vec<(&str, usize)> = vec![(start, 0)];
        let mut path: Vec<&str> = vec![start];
        state.insert(start, 1);
        while let Some((node, idx)) = stack.last_mut() {
            let edges = graph.get(node).map(Vec::as_slice).unwrap_or(&[]);
            if *idx < edges.len() {
                let next = edges[*idx];
                *idx += 1;
                match state.get(next) {
                    None if graph.contains_key(next) => {
                        state.insert(next, 1);
                        stack.push((next, 0));
                        path.push(next);
                    }
                    Some(1) => {
                        let pos = path.iter().position(|p| *p == next).unwrap();
                        let mut cycle: Vec<&str> = path[pos..].to_vec();
                        let mut key = cycle.clone();
                        key.sort();
                        if reported.insert(key) {
                            cycle.push(next);
                            issues.push(Issue::new(
                                Severity::Error,
                                "circular_requirement",
                                cycle[0],
                                format!("These entries require each other, so none of them can ever be satisfied: {}", cycle.join(" → ")),
                            ));
                        }
                    }
                    _ => {}
                }
            } else {
                state.insert(node, 2);
                stack.pop();
                path.pop();
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Entry, Trick};

    fn file(entries: Vec<Entry>) -> LogicFile {
        LogicFile::new(entries)
    }

    fn codes(issues: &[Issue]) -> Vec<&'static str> {
        issues.iter().map(|i| i.code).filter(|c| *c != "missing_location").collect()
    }

    #[test]
    fn bundled_logic_has_no_errors() {
        for id in ["casual", "glitched"] {
            let f = crate::bundled(id).unwrap();
            let errors: Vec<_> = validate(&f, None).into_iter().filter(|i| i.severity == Severity::Error).collect();
            assert!(errors.is_empty(), "{id}: {errors:#?}");
        }
    }

    #[test]
    fn finds_unknown_and_self_references() {
        let mut a = Entry::new("Helper A");
        a.requires = vec!["Helper A".into(), "Nope".into()];
        let issues = validate(&file(vec![a]), None);
        assert_eq!(codes(&issues), vec!["self_reference", "unknown_reference"]);
    }

    #[test]
    fn finds_hard_cycles_but_not_alternatives() {
        let mut a = Entry::new("A");
        a.requires = vec!["B".into()];
        let mut b = Entry::new("B");
        b.any_of = vec![vec!["A".into()]];
        let mut c = Entry::new("C");
        c.any_of = vec![vec!["A".into()], vec![]];
        let issues = validate(&file(vec![a, b, c]), None);
        assert_eq!(codes(&issues).iter().filter(|c| **c == "circular_requirement").count(), 1);
    }

    #[test]
    fn flags_redundancy_and_missing_trick_text() {
        let mut a = Entry::new("A");
        a.requires = vec!["X".into()];
        a.any_of = vec![vec!["X".into()], vec!["Y".into(), "Z".into()], vec!["Y".into()]];
        let mut t = Entry::new("Some Trick");
        t.trick = Some(Trick::default());
        let entries = vec![a, t, Entry::new("X"), Entry::new("Y"), Entry::new("Z")];
        let c = codes(&validate(&file(entries), None));
        assert!(c.contains(&"redundant_requirement"));
        assert!(c.contains(&"redundant_group"));
        assert!(c.contains(&"trick_without_description"));
    }
}
