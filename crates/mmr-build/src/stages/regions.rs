//! Where things are, for hint text (the old `Region`, `RegionForDirectHint`,
//! `RegionAreaOfTemple` extensions).

use crate::enums;
use crate::plan::PlacedItem;
use crate::{BuildError, Builder};
use serde_json::Value;

impl Builder<'_> {
    /// The plan entry for an item, by id (`ItemList[item]`).
    pub(crate) fn obj(&self, item: &str) -> Result<PlacedItem, BuildError> {
        self.item_object(item).cloned().ok_or_else(|| BuildError::Data(format!("{item} isn't in the plan")))
    }

    /// The region a location is in (`Item.Region(itemList)`): its own, or that
    /// of the place its reference (e.g. a grotto's entrance) leads to.
    pub(crate) fn region_of(&self, location: &str) -> Option<String> {
        let a = enums::items().attr(location, "Region")?;
        if let Some(r) = a.get("Region").as_str() {
            return Some(r.to_string());
        }
        let reference = a.get("Reference").as_str()?;
        let target = self.item_object(reference).and_then(|io| io.location.clone()).unwrap_or_else(|| reference.to_string());
        self.region_of(&target)
    }

    /// `RegionForDirectHint`: multi-location items use their first location.
    pub(crate) fn region_for_direct_hint(&self, location: &str) -> String {
        let first = enums::items()
            .attr(location, "MultiLocation")
            .and_then(|a| a.get("Locations").as_array()?.first()?.as_str().map(String::from))
            .unwrap_or_else(|| location.to_string());
        self.region_of(&first).unwrap_or_default()
    }

    /// (preposition with a trailing space, region name) for "somewhere in
    /// the Swamp"-style hints. The name is only given with a preposition.
    pub(crate) fn region_hint(&self, location: &str) -> (Option<String>, Option<String>) {
        let region = self.region_for_direct_hint(location);
        let preposition = region_preposition(&region);
        let name = preposition.as_ref().and_then(|_| region_name(&region));
        let preposition = preposition.map(|p| if p.trim().is_empty() { p } else { format!("{p} ") });
        (preposition, name)
    }

    /// `RegionAreaOfTemple`: the area, with temples following their
    /// (possibly shuffled) entrance.
    pub(crate) fn region_area_of_temple(&self, location: &str) -> Option<String> {
        let area = enums::items()
            .attr(location, "RegionArea")
            .and_then(|a| a.get("RegionArea").as_str().map(String::from))
            .or_else(|| self.region_of(location).and_then(|r| region_area(&r)))?;
        const TEMPLES: [(&str, &str); 4] = [
            ("Swamp", "AreaWoodFallTempleAccess"),
            ("Mountain", "AreaSnowheadTempleAccess"),
            ("Ocean", "AreaGreatBayTempleAccess"),
            ("Canyon", "AreaInvertedStoneTowerTempleAccess"),
        ];
        let Some(&(_, entrance)) = TEMPLES.iter().find(|(a, _)| *a == area) else { return Some(area) };
        let new_entrance = self.item_object(entrance).and_then(|io| io.location.clone()).unwrap_or_else(|| entrance.to_string());
        TEMPLES.iter().find(|(_, e)| *e == new_entrance).map(|(a, _)| a.to_string())
    }
}

fn region_attr(region: &str, field: &str) -> Option<String> {
    enums::get("Region").attr(region, "RegionName").and_then(|a| match a.get(field) {
        Value::String(s) => Some(s.clone()),
        _ => None,
    })
}

/// A region's display name.
pub(crate) fn region_name(region: &str) -> Option<String> {
    region_attr(region, "Name")
}

/// "in the", "on", ... (`Prepositsion` in the old code).
pub(crate) fn region_preposition(region: &str) -> Option<String> {
    region_attr(region, "Prepositsion")
}

pub(crate) fn region_area(region: &str) -> Option<String> {
    enums::get("Region").attr(region, "RegionArea").and_then(|a| a.get("RegionArea").as_str().map(String::from))
}
