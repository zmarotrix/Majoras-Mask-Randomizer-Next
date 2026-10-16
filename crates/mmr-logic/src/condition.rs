//! Setting conditions: when a logic entry is satisfied by a setting.
//!
//! Legacy logic stored these as C# source (`settings.DamageMode ==
//! DamageMode.Default`) compiled at runtime. They are now structured data:
//!
//! ```json
//! {"setting": "CloseCows"}                          // boolean setting is on
//! {"setting": "DamageMode", "is": "Default"}        // enum setting equals
//! {"setting": "EnemyMode", "has": "Randomized"}     // flags setting contains
//! {"randomized": "ItemGreatBayBossKey"}             // location is randomized
//! {"not": { ... }}
//! ```

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SettingCondition {
    Not { not: Box<SettingCondition> },
    Randomized { randomized: String },
    Setting {
        setting: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        is: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        has: Option<String>,
    },
}

/// Settings key holding the list of randomized locations.
pub const RANDOMIZED_LOCATIONS_KEY: &str = "CustomItemList";

impl SettingCondition {
    pub fn not(inner: SettingCondition) -> Self {
        SettingCondition::Not { not: Box::new(inner) }
    }

    /// The setting this condition reads.
    pub fn setting_key(&self) -> &str {
        match self {
            SettingCondition::Not { not } => not.setting_key(),
            SettingCondition::Randomized { .. } => RANDOMIZED_LOCATIONS_KEY,
            SettingCondition::Setting { setting, .. } => setting,
        }
    }

    /// Evaluate against setting values. Missing settings count as unset.
    pub fn eval(&self, values: &Map<String, Value>) -> bool {
        match self {
            SettingCondition::Not { not } => !not.eval(values),
            SettingCondition::Randomized { randomized } => values
                .get(RANDOMIZED_LOCATIONS_KEY)
                .and_then(Value::as_array)
                .is_some_and(|list| list.iter().any(|v| v.as_str() == Some(randomized))),
            SettingCondition::Setting { setting, is, has } => {
                let value = values.get(setting);
                match (is, has) {
                    (Some(expected), _) => value == Some(expected),
                    (None, Some(flag)) => value
                        .and_then(Value::as_array)
                        .is_some_and(|flags| flags.iter().any(|f| f.as_str() == Some(flag))),
                    (None, None) => value.and_then(Value::as_bool).unwrap_or(false),
                }
            }
        }
    }

    /// Parse a legacy C# setting expression.
    pub fn from_legacy(src: &str) -> Result<Self, ConditionError> {
        let err = || ConditionError(src.to_string());
        let s = src.trim();
        if let Some(rest) = s.strip_prefix('!') {
            return Ok(SettingCondition::not(Self::from_legacy(rest).map_err(|_| err())?));
        }
        let rest = s.strip_prefix("settings.").ok_or_else(err)?;
        for (op, negate) in [("!=", true), ("==", false)] {
            if let Some((lhs, rhs)) = rest.split_once(op) {
                let value = enum_member(rhs).ok_or_else(err)?;
                let cond = SettingCondition::Setting { setting: lhs.trim().to_string(), is: Some(Value::String(value)), has: None };
                return Ok(if negate { SettingCondition::not(cond) } else { cond });
            }
        }
        if let Some((lhs, call)) = rest.split_once(".HasFlag(") {
            let flag = enum_member(call.strip_suffix(')').ok_or_else(err)?).ok_or_else(err)?;
            return Ok(SettingCondition::Setting { setting: lhs.to_string(), is: None, has: Some(flag) });
        }
        if let Some((lhs, call)) = rest.split_once(".Contains(") {
            if lhs != RANDOMIZED_LOCATIONS_KEY {
                return Err(err());
            }
            let item = enum_member(call.strip_suffix(')').ok_or_else(err)?).ok_or_else(err)?;
            return Ok(SettingCondition::Randomized { randomized: item });
        }
        if rest.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            return Ok(SettingCondition::Setting { setting: rest.to_string(), is: None, has: None });
        }
        Err(err())
    }

    /// Render as a legacy C# setting expression. `enum_type` maps a setting
    /// key to its C# enum type name (needed for `==` and `HasFlag`).
    pub fn to_legacy(&self, enum_type: &dyn Fn(&str) -> Option<String>) -> String {
        match self {
            SettingCondition::Not { not } => match not.as_ref() {
                SettingCondition::Setting { setting, is: Some(v), has: None } => {
                    format!("settings.{setting} != {}", legacy_enum_value(setting, v, enum_type))
                }
                inner => format!("!{}", inner.to_legacy(enum_type)),
            },
            SettingCondition::Randomized { randomized } => {
                format!("settings.{RANDOMIZED_LOCATIONS_KEY}.Contains(Item.{randomized})")
            }
            SettingCondition::Setting { setting, is: Some(v), .. } => {
                format!("settings.{setting} == {}", legacy_enum_value(setting, v, enum_type))
            }
            SettingCondition::Setting { setting, has: Some(flag), .. } => {
                let ty = enum_type(setting).unwrap_or_else(|| setting.rsplit('.').next().unwrap().to_string());
                format!("settings.{setting}.HasFlag({ty}.{flag})")
            }
            SettingCondition::Setting { setting, .. } => format!("settings.{setting}"),
        }
    }
}

fn legacy_enum_value(setting: &str, v: &Value, enum_type: &dyn Fn(&str) -> Option<String>) -> String {
    match v {
        Value::String(s) => {
            let ty = enum_type(setting).unwrap_or_else(|| setting.rsplit('.').next().unwrap().to_string());
            format!("{ty}.{s}")
        }
        other => other.to_string(),
    }
}

/// `DamageMode.Default` -> `Default`; also accepts `true`/`false`/numbers.
fn enum_member(s: &str) -> Option<String> {
    let s = s.trim();
    let member = s.rsplit('.').next()?;
    (!member.is_empty() && member.chars().all(|c| c.is_alphanumeric() || c == '_')).then(|| member.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unsupported setting expression {0:?}")]
pub struct ConditionError(pub String);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(s: &str) -> SettingCondition {
        SettingCondition::from_legacy(s).unwrap()
    }

    #[test]
    fn parses_every_legacy_form() {
        assert_eq!(parse("settings.CloseCows"), SettingCondition::Setting { setting: "CloseCows".into(), is: None, has: None });
        assert_eq!(
            parse("!settings.EnemyMode.HasFlag(EnemyMode.Randomized)"),
            SettingCondition::not(SettingCondition::Setting { setting: "EnemyMode".into(), is: None, has: Some("Randomized".into()) })
        );
        assert_eq!(
            parse("settings.Character != Character.AdultLink"),
            SettingCondition::not(SettingCondition::Setting { setting: "Character".into(), is: Some(json!("AdultLink")), has: None })
        );
        assert_eq!(parse("settings.CustomItemList.Contains(Item.BottleCatchSpringWater)"), SettingCondition::Randomized { randomized: "BottleCatchSpringWater".into() });
        assert!(SettingCondition::from_legacy("settings.Foo(1)").is_err());
    }

    #[test]
    fn round_trips_through_legacy() {
        let types = |k: &str| Some(k.to_string());
        for src in [
            "settings.CloseCows",
            "!settings.TakeDamageWhileShielding",
            "settings.DamageMode == DamageMode.Default",
            "settings.Character != Character.AdultLink",
            "!settings.EnemyMode.HasFlag(EnemyMode.Randomized)",
            "!settings.CustomItemList.Contains(Item.ItemGreatBayBossKey)",
        ] {
            assert_eq!(parse(src).to_legacy(&types), src);
        }
    }

    #[test]
    fn evaluates() {
        let values = json!({
            "CloseCows": true,
            "DamageMode": "Default",
            "EnemyMode": ["Hyper"],
            "CustomItemList": ["ItemBow"],
        });
        let values = values.as_object().unwrap();
        assert!(parse("settings.CloseCows").eval(values));
        assert!(!parse("settings.IronGoron").eval(values));
        assert!(parse("settings.DamageMode == DamageMode.Default").eval(values));
        assert!(parse("!settings.EnemyMode.HasFlag(EnemyMode.Randomized)").eval(values));
        assert!(parse("settings.CustomItemList.Contains(Item.ItemBow)").eval(values));
    }

    #[test]
    fn serializes_compactly() {
        let c = parse("settings.DamageMode == DamageMode.Default");
        assert_eq!(serde_json::to_value(&c).unwrap(), json!({"setting": "DamageMode", "is": "Default"}));
        let back: SettingCondition = serde_json::from_value(json!({"not": {"randomized": "ItemBow"}})).unwrap();
        assert_eq!(back, SettingCondition::not(SettingCondition::Randomized { randomized: "ItemBow".into() }));
    }
}
