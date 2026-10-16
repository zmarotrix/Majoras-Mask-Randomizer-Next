use mmr_randomizer::{generate, importance::importance, Seed, Style};

#[test]
fn importance_is_consistent() {
    let logic = mmr_logic::bundled("casual").unwrap();
    let p = mmr_settings::presets().into_iter().find(|p| p.id == "season-2-standard").unwrap();
    let values = mmr_settings::apply_preset(&p, None);
    let out = generate(&logic, &values, Seed(11), Style::Random).unwrap();
    let t = std::time::Instant::now();
    let stones: Vec<String> = ["GossipTerminaSouth", "HintGaroCanyonLower1"].iter().map(|s| s.to_string()).collect();
    let imp = importance(&logic, &values, &out, &stones);
    eprintln!("importance in {} ms: {} required, {} important; {:?}", t.elapsed().as_millis(), imp.required.len(), imp.important.len(), imp.required_for);
    assert!(imp.required.is_subset(&imp.important));
    assert!(!imp.required.is_empty());
    let req_items = mmr_randomizer::required_items(&logic, &values, &out);
    assert_eq!(req_items.len(), imp.required.len());
}
