use std::collections::HashSet;

use atriumd::ha::prefs;
use atriumd::ha::registry::{AreaPrefs, Registry};
use serde_json::Value;

fn load() -> Option<(Registry, Vec<String>, Value)> {
    let path = std::env::var("ATRIUM_FIXTURE").ok()?;
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("ATRIUM_FIXTURE={path} could not be read: {e}"));
    let dump: Value = serde_json::from_str(&raw).expect("fixture is not valid JSON");

    fn parse<T: serde::de::DeserializeOwned>(dump: &Value, key: &str) -> Vec<T> {
        serde_json::from_value(dump.get(key).cloned().unwrap_or(Value::Array(vec![])))
            .unwrap_or_else(|e| panic!("{key} in fixture did not parse: {e}"))
    }
    let registry = Registry {
        areas: parse(&dump, "areas"),
        devices: parse(&dump, "devices"),
        entities: parse(&dump, "entities"),
    };
    let live: Vec<String> = dump
        .get("states")
        .and_then(Value::as_array)
        .map(|s| {
            s.iter()
                .filter_map(|e| e.get("entity_id").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Some((registry, live, dump))
}

#[test]
fn projection_is_sane_on_a_real_instance() {
    let Some((registry, live, _)) = load() else {
        eprintln!("skipped: set ATRIUM_FIXTURE to run");
        return;
    };
    let tabs = registry.project_tabs(&live, &AreaPrefs::default(), |id| id.to_string());
    assert!(!tabs.is_empty(), "a real instance must produce tabs with no favorites set");

    let mut seen = HashSet::new();
    for tab in &tabs {
        assert!(seen.insert(tab.id.clone()), "duplicate tab id {}", tab.id);
        assert!(!tab.title.is_empty(), "tab {} has no title", tab.id);
        assert!(!tab.icon.is_empty(), "tab {} has no icon", tab.id);
    }

    for tab in tabs.iter().filter(|t| t.id.starts_with("area:")) {
        assert!(!tab.entity_ids.is_empty(), "empty area tab {} leaked", tab.id);
    }

    let mut placed = HashSet::new();
    for tab in &tabs {
        for id in &tab.entity_ids {
            assert!(placed.insert(id.clone()), "{id} landed in two tabs");
        }
    }
}

#[test]
fn imported_dashboard_preferences_reproduce_the_overview() {
    let Some((registry, live, dump)) = load() else {
        eprintln!("skipped: set ATRIUM_FIXTURE to run");
        return;
    };
    let Some(config) = dump.get("default_config") else {
        eprintln!("skipped: fixture has no default_config");
        return;
    };
    let Some(imported) = prefs::from_lovelace_config(config, true) else {
        eprintln!("skipped: instance does not use a generated dashboard");
        return;
    };

    let tabs = registry.project_tabs(&live, &imported, |id| id.to_string());
    let hidden: HashSet<&str> = imported.hidden.iter().map(String::as_str).collect();

    for tab in &tabs {
        if let Some(area) = tab.id.strip_prefix("area:") {
            assert!(!hidden.contains(area), "{area} is hidden in Home Assistant but shown here");
        }
    }
    if imported.hide_entities_without_area {
        assert!(
            !tabs.iter().any(|t| t.id == "unassigned"),
            "instance hides arealess entities; Atrium must not show them"
        );
    }

    let shown: Vec<&str> = tabs
        .iter()
        .filter_map(|t| t.id.strip_prefix("area:"))
        .collect();
    let expected: Vec<&str> = imported
        .order
        .iter()
        .map(String::as_str)
        .filter(|a| shown.contains(a))
        .collect();
    let actual: Vec<&str> = shown
        .iter()
        .copied()
        .filter(|a| expected.contains(a))
        .collect();
    assert_eq!(actual, expected, "imported area order was not reproduced");

    eprintln!(
        "real instance: {} tabs, {} entities",
        tabs.len(),
        tabs.iter().map(|t| t.entity_ids.len()).sum::<usize>()
    );
}
