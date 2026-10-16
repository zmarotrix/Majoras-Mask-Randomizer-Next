//! Time-of-day and temporary item rules in placement reachability.

use mmr_randomizer::reach::Graph;
use mmr_randomizer::world::World;
use serde_json::{json, Value};

fn values(shuffled: &[&str], extra: &[(&str, Value)]) -> mmr_settings::Values {
    let mut v = mmr_settings::defaults();
    v.insert("CustomItemList".into(), json!(shuffled));
    for (k, x) in extra {
        v.insert(k.to_string(), x.clone());
    }
    v
}

/// Can `check` be reached with `swaps` applied to the vanilla placement?
fn reachable(values: &mmr_settings::Values, swaps: &[(&str, &str)], check: &str) -> bool {
    let game = mmr_data::game();
    let logic = mmr_logic::settings_rules::specialize(&mmr_logic::bundled("casual").unwrap(), values);
    let world = World::new(game, &logic, values);
    let graph = Graph::new(&world, &logic, values);
    let mut placement: Vec<Option<usize>> = (0..world.items.len()).map(|l| world.items[l].location.is_some().then_some(l)).collect();
    for (a, b) in swaps {
        let (a, b) = (world.id(a).unwrap(), world.id(b).unwrap());
        placement.swap(a, b);
    }
    let reach = graph.sweep(&world.starting, &placement, true);
    reach.locations[world.id(check).unwrap()]
}

#[test]
fn vanilla_game_is_beatable_with_time() {
    let game = mmr_data::game();
    let v = values(&[], &[]);
    let logic = mmr_logic::settings_rules::specialize(&mmr_logic::bundled("casual").unwrap(), &v);
    let world = World::new(game, &logic, &v);
    let graph = Graph::new(&world, &logic, &v);
    let placement: Vec<Option<usize>> = (0..world.items.len()).map(|l| world.items[l].location.is_some().then_some(l)).collect();
    let reach = graph.sweep(&world.starting, &placement, true);
    assert!(graph.goal_reached(&reach));
    // Time-limited quests work in the vanilla game.
    for check in [
        "MaskCouple",
        "MaskKeaton",
        "MaskBlast",
        "MaskAllNight",
        "HeartPieceNotebookGran2",
        "UpgradeGildedSword",
        "MaskPostmanHat",
        "SongLullabyIntro",
        "HeartPieceDekuPlayground",
        "HeartPieceHoneyAndDarling",
    ] {
        assert!(reach.locations[world.id(check).unwrap()], "{check}");
    }
}

#[test]
fn temporary_item_needed_early_is_useless_late() {
    // The Letter to Kafei must be posted by Night 1. Found at the Curiosity
    // Shop on Night 3, it's no use...
    let swap = [("TradeItemKafeiLetter", "MaskAllNight")];
    let v = values(&["TradeItemKafeiLetter", "MaskAllNight"], &[]);
    assert!(reachable(&v, &[], "MaskKeaton"));
    assert!(!reachable(&v, &swap, "MaskKeaton"));
    // ...unless quest items are kept through time.
    let kept = values(&["TradeItemKafeiLetter", "MaskAllNight"], &[("KeepQuestTradeThroughTime", json!(true))]);
    assert!(reachable(&kept, &swap, "MaskKeaton"));
}

#[test]
fn permanent_items_ignore_cycle_windows_after_time_travel() {
    // Keaton's mask (set up Day 1 / Night 1) found on Night 3 is still
    // usable next cycle: it's permanent.
    let v = values(&["MaskKeaton", "MaskAllNight"], &[]);
    let swaps = [("MaskKeaton", "MaskAllNight")];
    assert!(reachable(&v, &swaps, "MaskKeaton"));
}


/// Every check in the unshuffled game that ignoring time would reach must
/// also be reachable with time (the vanilla game can be fully completed).
#[test]
fn time_never_blocks_the_vanilla_game() {
    let game = mmr_data::game();
    for logic_id in ["casual", "glitched"] {
        let v = values(&[], &[]);
        let logic = mmr_logic::settings_rules::specialize(&mmr_logic::bundled(logic_id).unwrap(), &v);
        let world = World::new(game, &logic, &v);
        let graph = Graph::new(&world, &logic, &v);
        let placement: Vec<Option<usize>> = (0..world.items.len()).map(|l| world.items[l].location.is_some().then_some(l)).collect();
        let reach = graph.sweep(&world.starting, &placement, true);
        let mut scenario = mmr_logic::eval::Scenario { settings: v.clone(), ..Default::default() };
        scenario.items = world.starting.iter().map(|&i| world.items[i].id.clone()).collect();
        let timeless = mmr_logic::eval::Evaluator::new(&logic).vanilla_playthrough(&scenario);
        let blocked: Vec<&String> = timeless.spheres.iter().flatten().filter(|id| world.id(id).is_some_and(|l| !reach.locations[l])).collect();
        assert!(blocked.is_empty(), "{logic_id}: {} blocked by time: {blocked:?}", blocked.len());
    }
}
