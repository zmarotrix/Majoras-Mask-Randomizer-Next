//! Prove two logic files behave the same: evaluate both on many random
//! situations (inventories, tricks, settings) and compare every entry.

use crate::eval::{Evaluator, Scenario};
use crate::model::LogicFile;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize)]
pub struct Mismatch {
    pub id: String,
    /// Which scenario (index) disagreed, and how.
    pub scenario: usize,
    pub old: bool,
    pub new: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Comparison {
    pub scenarios: usize,
    pub compared_ids: usize,
    pub mismatches: Vec<Mismatch>,
}

impl Comparison {
    pub fn is_equivalent(&self) -> bool {
        self.mismatches.is_empty()
    }
}

/// Build `n` reproducible scenarios: random inventories of varying size,
/// random trick sets, and the given settings (cycled).
pub fn scenarios(n: usize, seed: u64, items: &[String], tricks: &[String], settings: &[Map<String, Value>]) -> Vec<Scenario> {
    let mut state = seed | 1;
    let mut next = move || {
        // xorshift64*: plenty for picking test cases.
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_f491_4f6c_dd1d)
    };
    (0..n)
        .map(|i| {
            // Sweep inventory density from empty to full so both sparse and
            // nearly-complete inventories are covered.
            let density = (i % 11) as u64 * 10; // 0%..100%
            let pick = |next: &mut dyn FnMut() -> u64, pct: u64| next() % 100 < pct;
            let items = items.iter().filter(|_| pick(&mut next, density)).cloned().collect();
            let trick_pct = next() % 101;
            let tricks = tricks.iter().filter(|_| pick(&mut next, trick_pct)).cloned().collect();
            let settings = if settings.is_empty() { Map::new() } else { settings[i % settings.len()].clone() };
            Scenario { items, tricks, settings }
        })
        .collect()
}

/// Compare `old` and `new` on every scenario, for every id in `ids`.
pub fn compare(old: &LogicFile, new: &LogicFile, ids: &BTreeSet<String>, scenarios: &[Scenario]) -> Comparison {
    let pairs: Vec<(String, String)> = ids.iter().map(|i| (i.clone(), i.clone())).collect();
    compare_pairs(old, new, &pairs, scenarios)
}

/// Compare `old` and `new` on every scenario: each `(old id, new id)` pair
/// must agree (ids differ when an entry was replaced, e.g. by a region).
pub fn compare_pairs(old: &LogicFile, new: &LogicFile, pairs: &[(String, String)], scenarios: &[Scenario]) -> Comparison {
    let a = Evaluator::new(old);
    let b = Evaluator::new(new);
    let mut mismatches = Vec::new();
    for (i, s) in scenarios.iter().enumerate() {
        let ra = a.evaluate(s);
        let rb = b.evaluate(s);
        for (old_id, new_id) in pairs {
            let (x, y) = (ra.is_satisfied(old_id), rb.is_satisfied(new_id));
            if x != y {
                mismatches.push(Mismatch { id: old_id.clone(), scenario: i, old: x, new: y });
            }
        }
    }
    Comparison { scenarios: scenarios.len(), compared_ids: pairs.len(), mismatches }
}
