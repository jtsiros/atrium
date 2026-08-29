use serde_json::Value;

use crate::ha::registry::AreaPrefs;

pub fn from_lovelace_config(config: &Value, current_hide_empty: bool) -> Option<AreaPrefs> {
    let strategy = config.get("strategy")?;
    let kind = strategy.get("type").and_then(Value::as_str)?;
    if !matches!(kind, "original-states" | "areas" | "home") {
        return None;
    }

    let areas = strategy.get("areas");
    let string_list = |key: &str| -> Vec<String> {
        areas
            .and_then(|a| a.get(key))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    };

    Some(AreaPrefs {
        order: string_list("order"),
        hidden: string_list("hidden"),
        hide_entities_without_area: strategy
            .get("hide_entities_without_area")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        // The dashboard has no equivalent setting, so the user's own choice is
        // carried forward by the caller rather than reset here.
        hide_empty_areas: current_hide_empty,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn adopts_order_hidden_and_arealess_preference() {
        let config = json!({
            "strategy": {
                "type": "original-states",
                "areas": {
                    "hidden": ["downstairs_patio", "upstairs_patio"],
                    "order": ["garage", "office", "entrance"]
                },
                "hide_entities_without_area": true,
                "hide_energy": false
            }
        });
        let prefs = from_lovelace_config(&config, true).expect("strategy should be adopted");
        assert_eq!(prefs.order, ["garage", "office", "entrance"]);
        assert_eq!(prefs.hidden, ["downstairs_patio", "upstairs_patio"]);
        assert!(prefs.hide_entities_without_area);
    }

    #[test]
    fn hand_written_dashboards_are_left_alone() {
        let config = json!({ "views": [{ "title": "Home", "cards": [] }] });
        assert!(from_lovelace_config(&config, true).is_none());
    }

    #[test]
    fn unrelated_strategies_are_left_alone() {
        let config = json!({ "strategy": { "type": "map" } });
        assert!(from_lovelace_config(&config, true).is_none());
    }

    #[test]
    fn a_strategy_with_no_area_preferences_yet_is_still_adopted() {
        let config = json!({ "strategy": { "type": "original-states" } });
        let prefs = from_lovelace_config(&config, true).expect("adopted");
        assert!(prefs.order.is_empty());
        assert!(prefs.hidden.is_empty());
        assert!(!prefs.hide_entities_without_area);
    }

    #[test]
    fn an_import_keeps_the_users_own_empty_rooms_choice() {
        let config = json!({ "strategy": { "type": "original-states" } });
        assert!(!from_lovelace_config(&config, false).unwrap().hide_empty_areas);
        assert!(from_lovelace_config(&config, true).unwrap().hide_empty_areas);
    }

    #[test]
    fn malformed_preference_entries_are_discarded_not_fatal() {
        let config = json!({
            "strategy": {
                "type": "original-states",
                "areas": { "order": ["office", 42, null, ""], "hidden": "not-a-list" }
            }
        });
        let prefs = from_lovelace_config(&config, true).expect("adopted");
        assert_eq!(prefs.order, ["office"]);
        assert!(prefs.hidden.is_empty());
    }
}
