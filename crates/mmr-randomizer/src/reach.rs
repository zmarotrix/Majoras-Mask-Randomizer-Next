//! Fast reachability for placement: which locations can be reached with a
//! set of items, collecting the items found along the way.
//!
//! The logic is compiled once into index-based form. Each query is a single
//! worklist pass: an entry is only re-checked when something it depends on
//! changes, so a full sweep costs roughly one visit per requirement.
//!
//! # Time of day
//!
//! The semantics are MMR 2.0's (its placement dependency check), which the
//! logic was written against. Every value is a set of the six periods
//! (Day 1 .. Night 3, one bit each) of one three-day cycle:
//!
//! - An entry's window is its `setup` times, or else its `available` times,
//!   or else any time: when you have to be working toward it in the cycle.
//!   Its value is its window intersected with its requirements' values. So a
//!   chain of temporary things only works if their windows overlap (Couple's
//!   Mask: set up on Day 1, so the Letter and Pendant must be too).
//! - An item found at a location has that location's value. If it was
//!   shuffled there and has `needed` times, the location must be available
//!   at one of them, or the item is useless (MMR 2.0: "is needed at .. but ..
//!   is only available at ..").
//! - Once time travel is possible, permanent items are kept through the Song
//!   of Time: they hold at every period, breaking the chain. Temporary items
//!   (and entrances, events) never are. Before time travel everything happens
//!   in one cycle.
//! - What's found where there's no return (`no_return`: the Moon, the Clock
//!   Tower roof) only helps after the Song of Time, except time-travel items.
//! - Places with `opening_hours` (shop doors, grottos) must be open when
//!   the check is collected: their times are checked against its `available`
//!   times instead of joining the chain; what it takes to get there still
//!   does. (MMR 2.0 intersected them like the rest,
//!   which made e.g. the All-Night Mask, set up on Day 1 behind a door open
//!   only at night, impossible.)
//! - `temporary_requires` only apply when the item found is not kept.
//! - `before: [X]` entries are evaluated in a copy of the cycle where X never
//!   happens ("layers"); permanent items still carry over from earlier cycles.
//!
//! All values only grow as more items are owned, so the worklist settles.
//! (The physical order of events within a cycle isn't modelled beyond this;
//! that would change what the existing logic means.)

use crate::world::{Id, World};
use mmr_logic::{EntryKind, LogicFile};
use mmr_settings::Values;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Every period.
pub const ALL: u8 = 0b11_1111;


#[derive(Clone, Copy)]
enum Ref {
    Item(Id),
    Entry(usize),
    /// Unknown id: treated as satisfied (validation reports these).
    Free,
    /// Can never be met (ruled out by the settings).
    Never,
}

pub struct Graph {
    goal: Option<usize>,
    time_travel: Option<usize>,
    /// Number of layers: the real cycle plus one per `before` target.
    layers: usize,
    requires: Vec<Vec<Ref>>,
    groups: Vec<Vec<Vec<Ref>>>,
    temporary_requires: Vec<Vec<Ref>>,
    /// Trick enabled / setting condition met (always true for other kinds).
    gate: Vec<bool>,
    /// `setup`, else `available`, else any time.
    window: Vec<u8>,
    /// `available` (any time if unset), for the `needed` check.
    available: Vec<u8>,
    no_return: Vec<bool>,
    /// Entries whose times are opening hours.
    is_hours: Vec<bool>,
    /// Layer whose values an entry's requirements are read from (`before`).
    read_layer: Vec<Option<usize>>,
    /// The layer in which this entry never happens.
    absent_in: Vec<Option<usize>>,
    item_absent_in: BTreeMap<Id, usize>,
    /// For location entries, the location's item id.
    location_of: Vec<Option<Id>>,
    /// For locations, their logic entry.
    entry_of: BTreeMap<Id, usize>,
    /// Entries to re-check when an entry changes.
    entry_users: Vec<Vec<usize>>,
    /// Entries to re-check when an item changes.
    item_users: BTreeMap<Id, Vec<usize>>,
    /// Shuffled locations with no logic entry: always reachable.
    free_locations: Vec<Id>,
    /// Per item: `needed` times, kept through time, time-travel item.
    needed: Vec<u8>,
    kept: Vec<bool>,
    is_time_travel: Vec<bool>,
}

/// Result of a sweep.
pub struct Reach {
    pub entries: Vec<bool>,
    pub locations: Vec<bool>,
    pub owned: Vec<bool>,
    /// Per location: the periods of the cycle it can be worked toward in
    /// (with what's found there counted as temporary).
    pub times: Vec<u8>,
}

struct State {
    entry: Vec<u8>,
    /// An entry's requirements, before its own window.
    req: Vec<u8>,
    full: Vec<u8>,
    perm: Vec<u8>,
    item: Vec<u8>,
    owned: Vec<bool>,
    item_at: Vec<Option<Id>>,
    time_travel: bool,
}

impl Graph {
    pub fn new(world: &World, logic: &LogicFile, values: &Values) -> Graph {
        let n = logic.entries.len();
        let entry_index: BTreeMap<&str, usize> = logic.entries.iter().enumerate().map(|(i, e)| (e.id.as_str(), i)).collect();
        let tricks: BTreeSet<&str> = values
            .get("EnabledTricks")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        let to_ref = |r: &str| -> Ref {
            if r == mmr_logic::settings_rules::IMPOSSIBLE {
                return Ref::Never;
            }
            if mmr_logic::is_item_ref(r) {
                if let Some(i) = world.id(r) {
                    return Ref::Item(i);
                }
            }
            entry_index.get(r).map_or(Ref::Free, |&e| Ref::Entry(e))
        };

        let targets: Vec<&str> = {
            let set: BTreeSet<&str> = logic.entries.iter().flat_map(|e| e.before.iter().map(String::as_str)).collect();
            set.into_iter().collect()
        };
        let layer_of = |t: &str| targets.iter().position(|x| *x == t).map(|p| p + 1);

        let n_items = world.items.len();
        let mut g = Graph {
            goal: entry_index.get(mmr_logic::GOAL).copied(),
            time_travel: entry_index.get("OtherTimeTravel").copied(),
            layers: targets.len() + 1,
            requires: Vec::with_capacity(n),
            groups: Vec::with_capacity(n),
            temporary_requires: Vec::with_capacity(n),
            gate: Vec::with_capacity(n),
            window: Vec::with_capacity(n),
            available: Vec::with_capacity(n),
            no_return: Vec::with_capacity(n),
            is_hours: Vec::with_capacity(n),
            read_layer: Vec::with_capacity(n),
            absent_in: Vec::with_capacity(n),
            item_absent_in: BTreeMap::new(),
            location_of: Vec::with_capacity(n),
            entry_of: BTreeMap::new(),
            entry_users: vec![Vec::new(); n],
            item_users: BTreeMap::new(),
            free_locations: Vec::new(),
            needed: vec![ALL; n_items],
            kept: world.items.iter().map(|i| !i.temporary && !i.entrance && i.name.is_some()).collect(),
            is_time_travel: world.items.iter().map(|i| i.time_travel).collect(),
        };
        for (i, e) in logic.entries.iter().enumerate() {
            let req: Vec<Ref> = e.requires.iter().map(|r| to_ref(r)).collect();
            let grp: Vec<Vec<Ref>> = e.any_of.iter().map(|g| g.iter().map(|r| to_ref(r)).collect()).collect();
            let tmp: Vec<Ref> = e.temporary_requires.iter().map(|r| to_ref(r)).collect();
            for r in req.iter().chain(grp.iter().flatten()).chain(&tmp) {
                match *r {
                    Ref::Entry(t) => g.entry_users[t].push(i),
                    Ref::Item(t) => g.item_users.entry(t).or_default().push(i),
                    Ref::Free | Ref::Never => {}
                }
            }
            g.requires.push(req);
            g.groups.push(grp);
            g.temporary_requires.push(tmp);
            g.gate.push(match EntryKind::of(e) {
                EntryKind::Trick => tricks.contains(e.id.as_str()),
                EntryKind::Setting => e.setting.as_ref().is_some_and(|c| c.eval(values)),
                _ => true,
            });
            let available = if e.time.available.is_empty() { ALL } else { e.time.available.0 & ALL };
            g.available.push(available);
            g.window.push(if e.time.setup.is_empty() { available } else { e.time.setup.0 & ALL });
            g.no_return.push(e.time.no_return);
            g.is_hours.push(e.time.opening_hours);
            // One layer per entry for now: with several `before` targets the
            // first is used (only one exists in MMR 2.0 logic).
            g.read_layer.push(e.before.first().and_then(|t| layer_of(t)));
            g.absent_in.push(layer_of(&e.id));
            let item = world.id(&e.id);
            if let Some(it) = item {
                if !e.time.needed.is_empty() {
                    g.needed[it] = e.time.needed.0 & ALL;
                }
                if let Some(k) = layer_of(&e.id).filter(|_| mmr_logic::is_item_ref(&e.id)) {
                    g.item_absent_in.insert(it, k);
                }
            }
            let loc = item.filter(|&l| world.items[l].location.is_some());
            if let Some(l) = loc {
                g.entry_of.insert(l, i);
            }
            g.location_of.push(loc);
        }
        for users in g.entry_users.iter_mut().chain(g.item_users.values_mut()) {
            users.sort_unstable();
            users.dedup();
        }
        g.free_locations = world.shuffled.iter().copied().filter(|l| !g.entry_of.contains_key(l)).collect();
        g
    }

    /// A reference's value, for an entry collected at `available` times.
    fn mask(&self, st: &State, r: &Ref, k: usize, available: u8) -> u8 {
        match *r {
            Ref::Item(i) => st.item[i * self.layers + k],
            Ref::Entry(t) if self.is_hours[t] => {
                if self.window[t] & available != 0 {
                    st.req[t * self.layers + k]
                } else {
                    0
                }
            }
            Ref::Entry(t) => st.entry[t * self.layers + k],
            Ref::Free => ALL,
            Ref::Never => 0,
        }
    }

    fn all(&self, st: &State, refs: &[Ref], k: usize, available: u8) -> u8 {
        refs.iter().fold(ALL, |m, r| m & self.mask(st, r, k, available))
    }

    /// What item `item`, found at location `loc`, is worth in layer `k`.
    fn item_value(&self, st: &State, item: Id, loc: Id, k: usize) -> u8 {
        if st.owned[item] {
            return ALL;
        }
        if self.item_absent_in.get(&item) == Some(&k) {
            return 0;
        }
        let l = self.layers;
        let (full, perm0, available, no_return) = match self.entry_of.get(&loc) {
            Some(&e) => (st.full[e * l + k], st.perm[e * l], self.available[e], self.no_return[e]),
            None => (ALL, ALL, ALL, false),
        };
        if no_return && !self.is_time_travel[item] {
            return if st.time_travel && perm0 != 0 { ALL } else { 0 };
        }
        if self.kept[item] && st.time_travel {
            return if perm0 != 0 { ALL } else { 0 };
        }
        if item != loc && self.needed[item] & available == 0 {
            return 0;
        }
        full
    }

    /// Recompute the item at `loc`; queue its users if it changed.
    fn refresh_item(&self, st: &mut State, item: Id, loc: Id, queue: &mut VecDeque<usize>, queued: &mut [bool]) {
        let l = self.layers;
        let mut changed = false;
        for k in 0..l {
            let v = st.item[item * l + k] | self.item_value(st, item, loc, k);
            if v != st.item[item * l + k] {
                st.item[item * l + k] = v;
                changed = true;
            }
        }
        if changed {
            if let Some(users) = self.item_users.get(&item) {
                for &u in users {
                    if !queued[u] {
                        queued[u] = true;
                        queue.push_back(u);
                    }
                }
            }
        }
    }

    /// Reach everything possible starting from `owned`. If `collect`, items
    /// found at reached locations (per `placement`, indexed by location) are
    /// obtained and used to reach further.
    pub fn sweep(&self, owned: &[Id], placement: &[Option<Id>], collect: bool) -> Reach {
        let l = self.layers;
        let n = self.requires.len();
        let n_items = self.needed.len();
        let mut st = State {
            entry: vec![0; n * l],
            req: vec![0; n * l],
            full: vec![0; n * l],
            perm: vec![0; n * l],
            item: vec![0; n_items * l],
            owned: vec![false; n_items],
            item_at: vec![None; n_items],
            time_travel: false,
        };
        for &i in owned {
            st.owned[i] = true;
            st.item[i * l..(i + 1) * l].fill(ALL);
        }
        let mut reached = vec![false; n_items];
        let mut queue: VecDeque<usize> = (0..n).collect();
        let mut queued = vec![true; n];

        for &loc in &self.free_locations {
            reached[loc] = true;
            if collect {
                if let Some(item) = placement[loc] {
                    st.item_at[item] = Some(loc);
                    self.refresh_item(&mut st, item, loc, &mut queue, &mut queued);
                }
            }
        }

        while let Some(e) = queue.pop_front() {
            queued[e] = false;
            let mut changed = false;
            for k in 0..l {
                let kk = self.read_layer[e].unwrap_or(k);
                let mut req = 0;
                let a = self.available[e];
                if self.gate[e] && self.absent_in[e] != Some(k) {
                    let groups = &self.groups[e];
                    let any = if groups.is_empty() { ALL } else { groups.iter().fold(0, |m, g| m | self.all(&st, g, kk, a)) };
                    req = self.all(&st, &self.requires[e], kk, a) & any;
                }
                let w = self.window[e];
                let perm = w & req;
                let full = w & req & self.all(&st, &self.temporary_requires[e], kk, a);
                let i = e * l + k;
                let (perm, full) = (st.perm[i] | perm, st.full[i] | full);
                let value = st.entry[i] | full;
                let req = st.req[i] | req;
                if perm != st.perm[i] || full != st.full[i] || value != st.entry[i] || req != st.req[i] {
                    st.req[i] = req;
                    st.perm[i] = perm;
                    st.full[i] = full;
                    st.entry[i] = value;
                    changed = true;
                }
            }
            if !changed {
                continue;
            }
            for &u in &self.entry_users[e] {
                if !queued[u] {
                    queued[u] = true;
                    queue.push_back(u);
                }
            }
            if let Some(loc) = self.location_of[e] {
                if st.perm[e * l] != 0 {
                    reached[loc] = true;
                }
                if collect {
                    if let Some(item) = placement[loc] {
                        st.item_at[item] = Some(loc);
                        self.refresh_item(&mut st, item, loc, &mut queue, &mut queued);
                    }
                }
            }
            if Some(e) == self.time_travel && !st.time_travel && st.entry[e * l] != 0 {
                // Permanent items are now kept through the Song of Time.
                st.time_travel = true;
                for item in 0..n_items {
                    if let Some(loc) = st.item_at[item] {
                        self.refresh_item(&mut st, item, loc, &mut queue, &mut queued);
                    }
                }
            }
        }
        let mut times = vec![0; n_items];
        for (&loc, &e) in &self.entry_of {
            times[loc] = st.full[e * l];
        }
        for &loc in &self.free_locations {
            times[loc] = ALL;
        }
        Reach {
            entries: (0..n).map(|e| st.entry[e * l] != 0).collect(),
            locations: reached,
            owned: (0..n_items).map(|i| st.item[i * l] != 0).collect(),
            times,
        }
    }

    pub fn goal_reached(&self, reach: &Reach) -> bool {
        self.goal.is_some_and(|g| reach.entries[g])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_periods() {
        assert_eq!(mmr_logic::TimeSet::ALL.0, ALL);
    }
}
