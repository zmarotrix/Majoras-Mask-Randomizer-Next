//! Random prices (the old `RandomizePrices`): shop, minigame and other
//! prices, and the wallets logic then needs to pay them.

use crate::rng::Rng;
use mmr_logic::{Entry, LogicFile};
use mmr_settings::Values;
use serde::Deserialize;
use serde_json::Value;
use std::sync::OnceLock;

/// Something the game charges for (`MessageCost`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MessageCost {
    #[serde(default)]
    pub name: Option<String>,
    pub category: String,
    pub cost: u16,
    #[serde(default)]
    pub locations_affected: Vec<String>,
    #[serde(default)]
    pub items_affected: Vec<String>,
}

/// Every price, in the order the ROM builder writes them.
pub fn message_costs() -> &'static [MessageCost] {
    static TABLE: OnceLock<Vec<MessageCost>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let text = include_str!("../../../data/rom/message_costs.json");
        serde_json::from_str(text.trim_start_matches('\u{FEFF}')).expect("bundled price table")
    })
}

fn flags<'v>(values: &'v Values, key: &str) -> Vec<&'v str> {
    values.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default()
}

const WALLET_200: &str = "MMRN Any Wallet";
const WALLET_500: &str = "MMRN Giant Or Royal Wallet";

/// Pick prices, and add the wallets they need to `logic` (unless there's
/// no logic). Returns one price per [`message_costs`] entry (`None`: left
/// as it is).
pub fn randomize(logic: &mut LogicFile, values: &Values, rng: &mut Rng) -> Vec<Option<u16>> {
    let mode = flags(values, "PriceMode");
    let has = |f: &str| mode.contains(&f);
    let custom = flags(values, "CustomItemList");
    let royal = has("AccountForRoyalWallet") && custom.contains(&"UpgradeRoyalWallet");
    let multiply = royal && (has("ShuffleOnly") || mode == ["AccountForRoyalWallet"]);
    let costs = message_costs();
    if !costs.iter().any(|c| has(&c.category)) && !royal {
        return vec![None; costs.len()];
    }

    let mut pool: Vec<u16> = costs.iter().filter(|c| has(&c.category)).map(|c| c.cost).collect();
    let random_price = |rng: &mut Rng, pool: &[u16]| -> u16 {
        if has("ShuffleOnly") {
            *rng.choose(pool).unwrap_or(&1)
        } else if royal {
            rng.weighted(&crate::price_tables::ROYAL) as u16 + 1
        } else {
            rng.weighted(&crate::price_tables::NORMAL) as u16 + 1
        }
    };
    let doubled = |c: u16| if multiply { (c << 1).min(999) } else { c };

    let mut out: Vec<Option<u16>> = Vec::with_capacity(costs.len());
    for c in costs {
        if !has(&c.category) && !royal {
            out.push(None);
            continue;
        }
        let cost = if has(&c.category) {
            let mut cost = random_price(rng, &pool);
            // The scrub's two prices must differ (purchase 1 comes right before).
            if c.name.as_deref() == Some("Business Scrub Purchase 2") {
                let first = out.last().copied().flatten().unwrap_or(150);
                for _ in 0..1000 {
                    if doubled(cost) != first {
                        break;
                    }
                    cost = random_price(rng, &pool);
                }
            }
            if let Some(p) = pool.iter().position(|&p| p == cost) {
                pool.remove(p);
            }
            cost
        } else {
            c.cost
        };
        out.push(Some(doubled(cost)));
    }

    let no_logic = values.get("LogicMode").and_then(Value::as_str) == Some("NoLogic");
    if !no_logic {
        add_wallet_logic(logic, values, &out);
    }
    out
}

fn add_wallet_logic(logic: &mut LogicFile, values: &Values, prices: &[Option<u16>]) {
    let any_wallet = find_or_add(logic, WALLET_200, &["UpgradeAdultWallet", "UpgradeGiantWallet", "UpgradeRoyalWallet"]);
    let big_wallet = find_or_add(logic, WALLET_500, &["UpgradeGiantWallet", "UpgradeRoyalWallet"]);
    let costs = message_costs();

    // Each location needs the wallet for its cheapest price.
    let mut cheapest: std::collections::BTreeMap<&str, u16> = Default::default();
    for (c, price) in costs.iter().zip(prices) {
        let Some(price) = *price else { continue };
        for location in &c.locations_affected {
            if price >= *cheapest.get(location.as_str()).unwrap_or(&u16::MAX) {
                continue;
            }
            cheapest.insert(location, price);
            if let Some(e) = logic.entries.iter_mut().find(|e| e.id == *location) {
                e.requires.retain(|r| *r != any_wallet && *r != big_wallet && r != "UpgradeRoyalWallet");
                let wallet = match price {
                    501.. => Some("UpgradeRoyalWallet".to_string()),
                    201.. => Some(big_wallet.clone()),
                    100.. => Some(any_wallet.clone()),
                    _ => None,
                };
                e.requires.extend(wallet);
            }
        }
    }

    // Anything needing a bought item needs the wallet to buy it.
    let keg_drops = values.get("KegDrops").and_then(Value::as_bool).unwrap_or(false);
    for (c, price) in costs.iter().zip(prices) {
        let wallet = match price {
            Some(201..) => "UpgradeGiantWallet".to_string(),
            Some(100..) => any_wallet.clone(),
            _ => continue,
        };
        for item in &c.items_affected {
            if item == "ItemPowderKeg" && keg_drops {
                continue;
            }
            for e in &mut logic.entries {
                if e.requires.contains(item) {
                    e.requires.push(wallet.clone());
                }
                for group in &mut e.any_of {
                    if group.contains(item) {
                        group.push(wallet.clone());
                    }
                }
            }
        }
    }
}

/// An entry that's any one of `wallets`: the logic's own if it has one.
fn find_or_add(logic: &mut LogicFile, id: &str, wallets: &[&str]) -> String {
    let mut want: Vec<Vec<String>> = wallets.iter().map(|w| vec![w.to_string()]).collect();
    want.sort();
    let existing = logic.entries.iter().find(|e| {
        let mut groups = e.any_of.clone();
        groups.sort();
        e.requires.is_empty() && e.setting.is_none() && e.trick.is_none() && groups == want
    });
    if let Some(e) = existing {
        return e.id.clone();
    }
    let mut e = Entry::new(id);
    e.any_of = wallets.iter().map(|w| vec![w.to_string()]).collect();
    logic.entries.push(e);
    id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::{Seed, Stream};

    #[test]
    fn prices_follow_the_settings() {
        let logic = mmr_logic::bundled("casual").unwrap();
        let mut values = mmr_settings::defaults();
        values.insert("PriceMode".into(), serde_json::json!(["Purchases", "Minigames", "Misc"]));
        let mut l = logic.clone();
        let prices = randomize(&mut l, &values, &mut Rng::new(Seed(5), Stream::Prices, 0));
        assert_eq!(prices.len(), message_costs().len());
        assert!(prices.iter().all(|p| p.is_some_and(|p| (1..=500).contains(&p))));
        // Something got expensive enough to need a wallet.
        assert!(l.entries.iter().any(|e| e.requires.iter().any(|r| r == "Any Wallet" || r.contains("Wallet"))));
        let again = randomize(&mut logic.clone(), &values, &mut Rng::new(Seed(5), Stream::Prices, 0));
        assert_eq!(prices, again);
    }
}
